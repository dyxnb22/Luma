//! macOS `networksetup` adapter for HTTP and SOCKS system proxy settings.
//!
//! All process execution is kept here. The adapter journals the pre-Luma values under LumaNext
//! and restores them only if the settings still match what Luma wrote.

use async_trait::async_trait;
use luma_application::{SystemProxyError, SystemProxyPort, SystemProxySetting, SystemProxyStatus};
use luma_storage::luma_next_support_dir;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::process::Command;
use tokio::sync::Mutex;

const MAX_COMMAND_OUTPUT: usize = 32 * 1024;

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

impl Default for MacSystemProxy {
    fn default() -> Self {
        Self::new()
    }
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

    async fn read_status(&self) -> Result<SystemProxyStatus, SystemProxyError> {
        let service = self.current_service().await?;
        let http = parse_setting(&run_networksetup(&["-getwebproxy", &service]).await?);
        let socks = parse_setting(&run_networksetup(&["-getsocksfirewallproxy", &service]).await?);
        Ok(SystemProxyStatus {
            service,
            http,
            socks,
        })
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
        let _guard = self.operation_lock.lock().await;
        self.read_status().await
    }

    async fn enable(
        &self,
        http_port: Option<u16>,
        socks_port: Option<u16>,
    ) -> Result<SystemProxyStatus, SystemProxyError> {
        let _guard = self.operation_lock.lock().await;
        if http_port.is_none() && socks_port.is_none() {
            return Err(SystemProxyError::InvalidInput {
                field: "ports".into(),
                message: "Mihomo exposes neither an HTTP nor a SOCKS port".into(),
            });
        }
        let before = self.read_status().await?;
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
        let _guard = self.operation_lock.lock().await;
        let current = self.read_status().await?;
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
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, bytes)
        .and_then(|_| std::fs::rename(&temp, &path))
        .map_err(|_| SystemProxyError::Unavailable("could not persist system proxy state".into()))
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
    let enabled = output.lines().any(|line| {
        let (key, value) = line.split_once(':').unwrap_or(("", ""));
        key.trim().eq_ignore_ascii_case("enabled")
            && matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "yes" | "on" | "1"
            )
    });
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

fn same_settings(a: &SystemProxyStatus, b: &SystemProxyStatus) -> bool {
    a.service == b.service && a.http == b.http && a.socks == b.socks
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
    fn selects_network_service_for_default_route_interface() {
        let order = "(1) Wi-Fi\n(Hardware Port: Wi-Fi, Device: en0)\n(2) USB 10/100/1000 LAN\n(Hardware Port: USB, Device: en5)\n";
        assert_eq!(
            service_for_interface(order, "en5").as_deref(),
            Some("USB 10/100/1000 LAN")
        );
        assert_eq!(service_for_interface(order, "en9"), None);
    }
}
