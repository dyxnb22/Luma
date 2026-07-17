//! macOS `networksetup` adapter for HTTP and SOCKS system proxy settings.
//!
//! All process execution is kept here. The adapter journals the pre-Luma values under LumaNext
//! and restores them only if the settings still match what Luma wrote.

use async_trait::async_trait;
use luma_application::{SystemProxyError, SystemProxyPort, SystemProxySetting, SystemProxyStatus};
use luma_storage::luma_next_support_dir;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::process::Command;
use tokio::sync::Mutex;

const MAX_COMMAND_OUTPUT: usize = 32 * 1024;
static JOURNAL_WRITE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub struct MacSystemProxy {
    configured_service: Option<String>,
    saved: Mutex<Option<SystemProxyStatus>>,
    applied: Mutex<Option<SystemProxyStatus>>,
    operation_lock: Mutex<()>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ProxyJournal {
    saved: SystemProxyStatus,
    applied: SystemProxyStatus,
}

/// Settings that Luma deliberately does not own because restoring them could require retaining
/// credentials or PAC URLs. When any are active, leave the service untouched.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct UnsupportedProxyFeatures {
    http_authentication: bool,
    secure_web_proxy: bool,
    socks_authentication: bool,
    auto_proxy_url: bool,
    proxy_auto_discovery: bool,
}

impl UnsupportedProxyFeatures {
    fn any(&self) -> bool {
        self.http_authentication
            || self.secure_web_proxy
            || self.socks_authentication
            || self.auto_proxy_url
            || self.proxy_auto_discovery
    }
}

struct SystemProxySnapshot {
    status: SystemProxyStatus,
    unsupported: UnsupportedProxyFeatures,
}

impl Default for MacSystemProxy {
    fn default() -> Self {
        Self::new()
    }
}

/// Holds the in-process mutex and the cross-process journal flock for one proxy operation.
struct SystemProxyOpGuard<'a> {
    _operation: tokio::sync::MutexGuard<'a, ()>,
    _file: Option<JournalLock>,
}

/// Cross-process advisory lock for the system-proxy journal. Mirrors settings.toml.lock:
/// keep the pathname after release so waiters share one inode.
struct JournalLock {
    _file: File,
}

impl JournalLock {
    fn acquire(journal_path: &Path) -> Result<Self, SystemProxyError> {
        let lock_path = journal_path.with_extension("json.lock");
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent).map_err(|_| {
                SystemProxyError::Unavailable("could not lock system proxy state".into())
            })?;
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&lock_path)
            .map_err(|_| {
                SystemProxyError::Unavailable("could not lock system proxy state".into())
            })?;
        flock_exclusive(&file)?;
        let _ = writeln!(file, "pid={}", std::process::id());
        let _ = file.sync_all();
        Ok(Self { _file: file })
    }
}

#[cfg(unix)]
fn flock_exclusive(file: &File) -> Result<(), SystemProxyError> {
    use std::os::unix::io::AsRawFd;
    extern "C" {
        fn flock(fd: std::os::fd::RawFd, operation: i32) -> i32;
    }
    const LOCK_EX: i32 = 0x2;
    let ret = unsafe { flock(file.as_raw_fd(), LOCK_EX) };
    if ret == 0 {
        Ok(())
    } else {
        Err(SystemProxyError::Unavailable(
            "could not lock system proxy state".into(),
        ))
    }
}

#[cfg(not(unix))]
fn flock_exclusive(_file: &File) -> Result<(), SystemProxyError> {
    Ok(())
}

impl MacSystemProxy {
    pub fn new() -> Self {
        let journal = load_journal();
        Self {
            configured_service: None,
            saved: Mutex::new(journal.as_ref().map(|j| j.saved.clone())),
            applied: Mutex::new(journal.map(|j| j.applied)),
            operation_lock: Mutex::new(()),
        }
    }

    pub fn with_service(service: Option<String>) -> Self {
        let mut adapter = Self::new();
        adapter.configured_service = service.filter(|service| !service.trim().is_empty());
        adapter
    }

    async fn lock_ops(&self) -> Result<SystemProxyOpGuard<'_>, SystemProxyError> {
        let operation = self.operation_lock.lock().await;
        let file = match journal_path() {
            Some(path) => Some(
                tokio::task::spawn_blocking(move || JournalLock::acquire(&path))
                    .await
                    .map_err(|err| {
                        SystemProxyError::Unavailable(format!("proxy lock task failed: {err}"))
                    })??,
            ),
            None => None,
        };
        Ok(SystemProxyOpGuard {
            _operation: operation,
            _file: file,
        })
    }

    /// Reload `saved`/`applied` from disk so a peer process's journal updates are visible.
    async fn hydrate_from_journal(&self) {
        let journal = load_journal();
        *self.saved.lock().await = journal.as_ref().map(|j| j.saved.clone());
        *self.applied.lock().await = journal.map(|j| j.applied);
    }

    /// If journal claims Luma ownership but live settings diverged (crash mid-apply),
    /// force-restore the pre-Luma `saved` snapshot and clear the journal.
    /// Returns `(live, restored)`.
    async fn reconcile_if_diverged(
        &self,
        live: SystemProxyStatus,
    ) -> Result<(SystemProxyStatus, bool), SystemProxyError> {
        let applied = self.applied.lock().await.clone();
        let Some(applied) = applied.as_ref() else {
            return Ok((live, false));
        };
        if same_settings(&live, applied) {
            return Ok((live, false));
        }
        // A changed service is an ownership ambiguity, not a half-apply. Never restore a
        // snapshot captured for one network service onto whichever service is current now.
        if live.service != applied.service {
            return Err(SystemProxyError::Conflict);
        }
        let Some(previous) = self.saved.lock().await.clone() else {
            return Err(SystemProxyError::Conflict);
        };
        if !should_force_restore(&live, applied, &previous) {
            return Err(SystemProxyError::Conflict);
        }
        self.apply_pair(&live.service, &previous.http, &previous.socks)
            .await?;
        *self.saved.lock().await = None;
        *self.applied.lock().await = None;
        clear_journal()?;
        Ok((self.read_status().await?, true))
    }

    async fn current_service(&self) -> Result<String, SystemProxyError> {
        if let Some(service) = &self.configured_service {
            return Ok(service.clone());
        }
        let interface = run_command("route", &["-n", "get", "default"])
            .await
            .ok()
            .and_then(|output| {
                output.lines().find_map(|line| {
                    let (key, value) = line.split_once(':')?;
                    key.trim()
                        .eq_ignore_ascii_case("interface")
                        .then(|| value.trim().to_string())
                })
            });
        if let Some(interface) = interface {
            if let Ok(order) = run_networksetup(&["-listnetworkserviceorder"]).await {
                if let Some(service) = service_for_interface(&order, &interface) {
                    return Ok(service);
                }
            }
        }
        let output = run_networksetup(&["-listallnetworkservices"]).await?;
        let service = output
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with("An asterisk"))
            .find(|line| !line.starts_with('*'))
            .ok_or_else(|| SystemProxyError::Unavailable("no active network service found".into()))?
            .to_string();
        Ok(service)
    }

    async fn read_snapshot(&self) -> Result<SystemProxySnapshot, SystemProxyError> {
        let service = self.current_service().await?;
        let http_output = run_networksetup(&["-getwebproxy", &service]).await?;
        let secure_web_output = run_networksetup(&["-getsecurewebproxy", &service]).await?;
        let socks_output = run_networksetup(&["-getsocksfirewallproxy", &service]).await?;
        let auto_proxy_url = run_networksetup(&["-getautoproxyurl", &service]).await?;
        let proxy_auto_discovery = run_networksetup(&["-getproxyautodiscovery", &service]).await?;
        Ok(SystemProxySnapshot {
            status: SystemProxyStatus {
                service,
                http: parse_setting(&http_output),
                socks: parse_setting(&socks_output),
            },
            unsupported: UnsupportedProxyFeatures {
                http_authentication: authenticated_proxy_enabled(&http_output),
                // HTTPS has an independent `networksetup` setting. Luma intentionally owns
                // only HTTP and SOCKS, so an enabled secure Web proxy makes a partial snapshot
                // unsafe to take over or restore.
                secure_web_proxy: enabled_setting(&secure_web_output),
                socks_authentication: authenticated_proxy_enabled(&socks_output),
                auto_proxy_url: enabled_setting(&auto_proxy_url),
                proxy_auto_discovery: enabled_setting(&proxy_auto_discovery),
            },
        })
    }

    async fn read_status(&self) -> Result<SystemProxyStatus, SystemProxyError> {
        let snapshot = self.read_snapshot().await?;
        if snapshot.unsupported.any() {
            return Err(SystemProxyError::Conflict);
        }
        Ok(snapshot.status)
    }

    async fn set_setting(
        service: &str,
        kind: &str,
        setting: &SystemProxySetting,
    ) -> Result<(), SystemProxyError> {
        let (set, state) = match kind {
            "http" => ("-setwebproxy", "-setwebproxystate"),
            "socks" => ("-setsocksfirewallproxy", "-setsocksfirewallproxystate"),
            _ => return Err(SystemProxyError::Unavailable("unknown proxy kind".into())),
        };
        if setting.enabled {
            let server = setting.server.as_deref().unwrap_or("127.0.0.1");
            let port = setting.port.unwrap_or(0);
            if port == 0 {
                return Err(SystemProxyError::InvalidInput {
                    field: format!("{kind}_port"),
                    message: "a port is required to enable the system proxy".into(),
                });
            }
            run_networksetup(&[set, service, server, &port.to_string()]).await?;
            run_networksetup(&[state, service, "on"]).await?;
        } else {
            run_networksetup(&[state, service, "off"]).await?;
        }
        Ok(())
    }

    async fn apply_pair(
        &self,
        service: &str,
        http: &SystemProxySetting,
        socks: &SystemProxySetting,
    ) -> Result<(), SystemProxyError> {
        Self::set_setting(service, "http", http).await?;
        Self::set_setting(service, "socks", socks).await?;
        Ok(())
    }
}

#[async_trait]
impl SystemProxyPort for MacSystemProxy {
    async fn get_status(&self) -> Result<SystemProxyStatus, SystemProxyError> {
        let _guard = self.lock_ops().await?;
        self.read_status().await
    }

    async fn enable(
        &self,
        http_port: Option<u16>,
        socks_port: Option<u16>,
    ) -> Result<SystemProxyStatus, SystemProxyError> {
        let _guard = self.lock_ops().await?;
        self.hydrate_from_journal().await;
        if http_port.is_none() && socks_port.is_none() {
            return Err(SystemProxyError::InvalidInput {
                field: "ports".into(),
                message: "Mihomo exposes neither an HTTP nor a SOCKS port".into(),
            });
        }
        let before = self.read_status().await?;
        let (before, _) = self.reconcile_if_diverged(before).await?;
        if let Some(applied) = self.applied.lock().await.clone() {
            if !same_settings(&before, &applied) {
                return Err(SystemProxyError::Conflict);
            }
        } else if self.saved.lock().await.is_some() {
            return Err(SystemProxyError::Conflict);
        }
        if self.saved.lock().await.is_none() {
            *self.saved.lock().await = Some(before.clone());
        }
        let wanted_http = http_port
            .map(|port| SystemProxySetting {
                enabled: true,
                server: Some("127.0.0.1".into()),
                port: Some(port),
            })
            .unwrap_or_else(|| before.http.clone());
        let wanted_socks = socks_port
            .map(|port| SystemProxySetting {
                enabled: true,
                server: Some("127.0.0.1".into()),
                port: Some(port),
            })
            .unwrap_or_else(|| before.socks.clone());
        let desired = SystemProxyStatus {
            service: before.service.clone(),
            http: wanted_http.clone(),
            socks: wanted_socks.clone(),
        };
        *self.applied.lock().await = Some(desired.clone());
        let saved = self.saved.lock().await.clone();
        persist_journal(&saved, &Some(desired))?;
        if let Err(error) = self
            .apply_pair(&before.service, &wanted_http, &wanted_socks)
            .await
        {
            let rollback = self
                .apply_pair(&before.service, &before.http, &before.socks)
                .await;
            if rollback.is_ok() {
                *self.saved.lock().await = None;
                *self.applied.lock().await = None;
                clear_journal()?;
            }
            return Err(if rollback.is_err() {
                SystemProxyError::Unavailable(
                    "system proxy partially changed; manual retry may be required".into(),
                )
            } else {
                error
            });
        }
        let after = self.read_status().await?;
        *self.applied.lock().await = Some(after.clone());
        let saved = self.saved.lock().await.clone();
        let applied = self.applied.lock().await.clone();
        persist_journal(&saved, &applied)?;
        Ok(after)
    }

    async fn disable(&self) -> Result<SystemProxyStatus, SystemProxyError> {
        let _guard = self.lock_ops().await?;
        self.hydrate_from_journal().await;
        let current = self.read_status().await?;
        let (current, restored) = self.reconcile_if_diverged(current).await?;
        if restored {
            return Ok(current);
        }
        let applied = self.applied.lock().await.clone();
        let Some(applied) = applied else {
            return Err(SystemProxyError::Conflict);
        };
        if !same_settings(&current, &applied) {
            return Err(SystemProxyError::Conflict);
        }
        let Some(previous) = self.saved.lock().await.clone() else {
            return Err(SystemProxyError::Conflict);
        };
        if let Err(error) = self
            .apply_pair(&current.service, &previous.http, &previous.socks)
            .await
        {
            let rollback = self
                .apply_pair(&current.service, &applied.http, &applied.socks)
                .await;
            return Err(if rollback.is_err() {
                SystemProxyError::Unavailable(
                    "system proxy partially changed; manual retry may be required".into(),
                )
            } else {
                error
            });
        }
        *self.saved.lock().await = None;
        *self.applied.lock().await = None;
        clear_journal()?;
        self.read_status().await
    }
}

async fn run_networksetup(args: &[&str]) -> Result<String, SystemProxyError> {
    let output = Command::new("networksetup")
        .args(args)
        .output()
        .await
        .map_err(|e| SystemProxyError::Unavailable(format!("networksetup unavailable: {e}")))?;
    if output.stdout.len() + output.stderr.len() > MAX_COMMAND_OUTPUT {
        return Err(SystemProxyError::Unavailable(
            "networksetup response exceeded the size limit".into(),
        ));
    }
    if !output.status.success() {
        let _ = output.stderr;
        return Err(SystemProxyError::PermissionRequired(
            "macOS rejected the network proxy operation; check Network settings permissions".into(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

async fn run_command(program: &str, args: &[&str]) -> Result<String, SystemProxyError> {
    let output = Command::new(program)
        .args(args)
        .output()
        .await
        .map_err(|e| SystemProxyError::Unavailable(format!("{program} unavailable: {e}")))?;
    if output.stdout.len() + output.stderr.len() > MAX_COMMAND_OUTPUT {
        return Err(SystemProxyError::Unavailable(
            "system command response exceeded the size limit".into(),
        ));
    }
    if !output.status.success() {
        return Err(SystemProxyError::Unavailable(format!(
            "{program} could not determine the active route"
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn service_for_interface(order: &str, interface: &str) -> Option<String> {
    let mut candidate = None;
    for line in order.lines().map(str::trim) {
        if let Some(numbered) = line.strip_prefix('(') {
            if let Some((number, name)) = numbered.split_once(')') {
                if number.chars().all(|ch| ch.is_ascii_digit()) {
                    let name = name.trim();
                    candidate = (!name.starts_with('*')).then(|| name.to_string());
                    continue;
                }
            }
        }
        if line.contains("Device:")
            && line.contains(&format!("Device: {interface}"))
            && candidate.is_some()
        {
            return candidate;
        }
    }
    None
}

fn journal_path() -> Option<PathBuf> {
    luma_next_support_dir()
        .ok()
        .map(|dir| dir.join("proxy-system-state.json"))
}

fn load_journal() -> Option<ProxyJournal> {
    let path = journal_path()?;
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn persist_journal(
    saved: &Option<SystemProxyStatus>,
    applied: &Option<SystemProxyStatus>,
) -> Result<(), SystemProxyError> {
    let (Some(saved), Some(applied), Some(path)) = (saved, applied, journal_path()) else {
        return Ok(());
    };
    let bytes = serde_json::to_vec(&ProxyJournal {
        saved: saved.clone(),
        applied: applied.clone(),
    })
    .map_err(|_| SystemProxyError::Unavailable("could not persist system proxy state".into()))?;
    write_journal_atomically(&path, &bytes)
}

/// Journal state is a rollback safety boundary, so never reuse a predictable staging path that
/// another process or stale directory entry could block. The same-directory rename is atomic.
fn write_journal_atomically(path: &Path, bytes: &[u8]) -> Result<(), SystemProxyError> {
    let parent = path.parent().ok_or_else(|| {
        SystemProxyError::Unavailable("could not persist system proxy state".into())
    })?;
    fs::create_dir_all(parent).map_err(|_| {
        SystemProxyError::Unavailable("could not persist system proxy state".into())
    })?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            SystemProxyError::Unavailable("could not persist system proxy state".into())
        })?;
    for _ in 0..16 {
        let sequence = JOURNAL_WRITE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary = path.with_file_name(format!(
            ".{file_name}.{}.{}.tmp",
            std::process::id(),
            sequence
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = match options.open(&temporary) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => {
                return Err(SystemProxyError::Unavailable(
                    "could not persist system proxy state".into(),
                ));
            }
        };
        let write_result = file.write_all(bytes).and_then(|_| file.sync_all());
        drop(file);
        if write_result.is_err() {
            let _ = fs::remove_file(&temporary);
            return Err(SystemProxyError::Unavailable(
                "could not persist system proxy state".into(),
            ));
        }
        if fs::rename(&temporary, path).is_err() {
            let _ = fs::remove_file(&temporary);
            return Err(SystemProxyError::Unavailable(
                "could not persist system proxy state".into(),
            ));
        }
        // The commit succeeded if rename succeeded. Do not retry it merely because a best-effort
        // directory sync is unsupported by this filesystem.
        let _ = fs::File::open(parent).and_then(|directory| directory.sync_all());
        return Ok(());
    }
    Err(SystemProxyError::Unavailable(
        "could not persist system proxy state".into(),
    ))
}

fn clear_journal() -> Result<(), SystemProxyError> {
    let Some(path) = journal_path() else {
        return Ok(());
    };
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(SystemProxyError::Unavailable(
            "could not clear system proxy state".into(),
        )),
    }
}

fn parse_setting(output: &str) -> SystemProxySetting {
    let enabled = enabled_setting(output);
    let server = output.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        (key.trim().eq_ignore_ascii_case("server") && !value.trim().is_empty())
            .then(|| value.trim().to_string())
    });
    let port = output.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.trim()
            .eq_ignore_ascii_case("port")
            .then(|| value.trim().parse::<u16>().ok())
            .flatten()
    });
    SystemProxySetting {
        enabled,
        server,
        port,
    }
}

fn enabled_setting(output: &str) -> bool {
    output.lines().any(|line| {
        let (key, value) = line.split_once(':').unwrap_or(("", ""));
        key.trim().eq_ignore_ascii_case("enabled")
            && matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "yes" | "on" | "1"
            )
    })
}

fn authenticated_proxy_enabled(output: &str) -> bool {
    output.lines().any(|line| {
        let (key, value) = line.split_once(':').unwrap_or(("", ""));
        key.trim()
            .eq_ignore_ascii_case("authenticated proxy enabled")
            && matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "yes" | "on" | "1"
            )
    })
}

fn same_settings(a: &SystemProxyStatus, b: &SystemProxyStatus) -> bool {
    a.service == b.service && a.http == b.http && a.socks == b.socks
}

fn should_force_restore(
    live: &SystemProxyStatus,
    applied: &SystemProxyStatus,
    saved: &SystemProxyStatus,
) -> bool {
    if live.service != applied.service || live.service != saved.service {
        return false;
    }
    // A journal can safely identify the ordinary pre-apply and first-setting half-apply states.
    // Any other divergence may be a user change while Luma owns the proxy, so leave it alone and
    // report Conflict rather than overwriting it with the saved snapshot.
    same_settings(live, saved)
        || (live.http == applied.http && live.socks == saved.socks)
        || (live.http == saved.http && live.socks == applied.socks)
}

#[cfg(test)]
fn apply_journal_memory(
    saved: &mut Option<SystemProxyStatus>,
    applied: &mut Option<SystemProxyStatus>,
    journal: Option<ProxyJournal>,
) {
    *saved = journal.as_ref().map(|j| j.saved.clone());
    *applied = journal.map(|j| j.applied);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_networksetup_output() {
        let setting = parse_setting("Enabled: Yes\nServer: 127.0.0.1\nPort: 7899\n");
        assert_eq!(
            setting,
            SystemProxySetting {
                enabled: true,
                server: Some("127.0.0.1".into()),
                port: Some(7899),
            }
        );
    }

    #[test]
    fn disabled_output_is_safe() {
        let setting = parse_setting("Enabled: No\nServer: 127.0.0.1\nPort: 7899\n");
        assert!(!setting.enabled);
    }

    #[test]
    fn detects_proxy_features_luma_cannot_safely_restore() {
        assert!(authenticated_proxy_enabled(
            "Enabled: Yes\nAuthenticated Proxy Enabled: 1\n"
        ));
        assert!(enabled_setting(
            "URL: https://pac.invalid/config\nEnabled: Yes\n"
        ));
        let features = UnsupportedProxyFeatures {
            http_authentication: true,
            secure_web_proxy: true,
            socks_authentication: false,
            auto_proxy_url: true,
            proxy_auto_discovery: false,
        };
        assert!(features.any());
    }

    #[test]
    fn selects_network_service_for_default_route_interface() {
        let order = "(1) Wi-Fi\n(Hardware Port: Wi-Fi, Device: en0)\n(2) USB 10/100/1000 LAN\n(Hardware Port: USB, Device: en5)\n";
        assert_eq!(
            service_for_interface(order, "en5").as_deref(),
            Some("USB 10/100/1000 LAN")
        );
        assert_eq!(service_for_interface(order, "en9"), None);
    }

    #[test]
    fn journal_write_does_not_reuse_legacy_fixed_temp_path() {
        let directory = tempfile::tempdir().expect("tempdir");
        let journal = directory.path().join("proxy-system-state.json");
        std::fs::create_dir(journal.with_extension("json.tmp")).expect("stale legacy temp");

        write_journal_atomically(&journal, b"{}\n").expect("atomic journal write");

        assert_eq!(std::fs::read(&journal).expect("journal"), b"{}\n");
    }

    #[test]
    fn diverged_applied_should_force_restore() {
        let saved = SystemProxyStatus {
            service: "Wi-Fi".into(),
            http: SystemProxySetting {
                enabled: false,
                server: None,
                port: None,
            },
            socks: SystemProxySetting {
                enabled: false,
                server: None,
                port: None,
            },
        };
        let applied = SystemProxyStatus {
            service: "Wi-Fi".into(),
            http: SystemProxySetting {
                enabled: true,
                server: Some("127.0.0.1".into()),
                port: Some(7890),
            },
            socks: SystemProxySetting {
                enabled: true,
                server: Some("127.0.0.1".into()),
                port: Some(7891),
            },
        };
        let half = SystemProxyStatus {
            service: "Wi-Fi".into(),
            http: applied.http.clone(),
            socks: saved.socks.clone(),
        };
        assert!(should_force_restore(&half, &applied, &saved));
        assert!(should_force_restore(&saved, &applied, &saved));
        assert!(!should_force_restore(&applied, &applied, &saved));
        let external = SystemProxyStatus {
            service: "Wi-Fi".into(),
            http: applied.http.clone(),
            socks: SystemProxySetting {
                enabled: true,
                server: Some("192.0.2.1".into()),
                port: Some(9999),
            },
        };
        assert!(!should_force_restore(&external, &applied, &saved));
    }

    #[test]
    fn hydrate_overwrites_stale_in_memory_state() {
        let journal = ProxyJournal {
            saved: SystemProxyStatus {
                service: "Wi-Fi".into(),
                http: SystemProxySetting {
                    enabled: false,
                    server: None,
                    port: None,
                },
                socks: SystemProxySetting {
                    enabled: false,
                    server: None,
                    port: None,
                },
            },
            applied: SystemProxyStatus {
                service: "Wi-Fi".into(),
                http: SystemProxySetting {
                    enabled: true,
                    server: Some("127.0.0.1".into()),
                    port: Some(1),
                },
                socks: SystemProxySetting {
                    enabled: false,
                    server: None,
                    port: None,
                },
            },
        };
        let mut saved = None;
        let mut applied = Some(SystemProxyStatus {
            service: "stale".into(),
            http: SystemProxySetting {
                enabled: true,
                server: Some("0.0.0.0".into()),
                port: Some(9),
            },
            socks: SystemProxySetting {
                enabled: false,
                server: None,
                port: None,
            },
        });
        apply_journal_memory(&mut saved, &mut applied, Some(journal.clone()));
        assert_eq!(saved.as_ref(), Some(&journal.saved));
        assert_eq!(applied.as_ref(), Some(&journal.applied));
        apply_journal_memory(&mut saved, &mut applied, None);
        assert!(saved.is_none());
        assert!(applied.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn journal_lock_blocks_a_second_nonblocking_contender() {
        use std::os::unix::io::AsRawFd;

        let directory = tempfile::tempdir().expect("tempdir");
        let journal = directory.path().join("proxy-system-state.json");
        let held = JournalLock::acquire(&journal).expect("journal lock");
        let lock_path = journal.with_extension("json.lock");
        assert!(lock_path.exists());

        let contender = OpenOptions::new()
            .write(true)
            .open(&lock_path)
            .expect("open contender");
        extern "C" {
            fn flock(fd: std::os::fd::RawFd, operation: i32) -> i32;
        }
        const LOCK_EX: i32 = 0x2;
        const LOCK_NB: i32 = 0x4;
        let ret = unsafe { flock(contender.as_raw_fd(), LOCK_EX | LOCK_NB) };
        assert_ne!(ret, 0, "second process must not take a parallel flock");
        drop(held);
        let ret = unsafe { flock(contender.as_raw_fd(), LOCK_EX | LOCK_NB) };
        assert_eq!(ret, 0, "lock should release with the holder");
    }
}
