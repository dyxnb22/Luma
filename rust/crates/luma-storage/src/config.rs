use crate::paths::{ensure_luma_next_dirs, luma_next_support_dir, PathsError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error(transparent)]
    Paths(#[from] PathsError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml deserialize: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("toml serialize: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("version conflict: expected {expected}, found {found}")]
    VersionConflict { expected: u64, found: u64 },
    #[error("corrupt config quarantined to {0}")]
    Corrupt(PathBuf),
    #[error("settings lock timeout — another Luma instance may be saving")]
    LockTimeout,
    #[error("settings mutation rejected: {0}")]
    Mutation(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportedProject {
    pub path: String,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LumaSettings {
    pub schema_version: u32,
    pub settings_version: u64,
    pub enabled_modules: BTreeMap<String, bool>,
    pub notes_root: Option<String>,
    #[serde(default)]
    pub projects_roots: Vec<String>,
    /// User-imported project directories (canonical paths).
    #[serde(default)]
    pub imported_projects: Vec<ImportedProject>,
    /// Glob patterns relative to notes_root (e.g. `private/*`).
    #[serde(default)]
    pub notes_exclude_patterns: Vec<String>,
    /// Markdown records import root (e.g. ~/Documents/Notes/Records).
    #[serde(default)]
    pub records_root: Option<String>,
    pub clipboard_retention_days: u32,
    /// Lock Secrets vault after this many idle seconds (`0` disables idle lock).
    #[serde(default = "default_secrets_idle_lock_secs")]
    pub secrets_idle_lock_secs: u32,
    /// Max window rows on the Hub (clamped 5–50 when applied).
    #[serde(default = "default_hub_windows_max")]
    pub hub_windows_max: u32,
    /// Optional loopback/Unix Mihomo controller settings. Secret is a Keychain account name,
    /// never the secret value itself.
    #[serde(default)]
    pub proxy_controller_unix_socket: Option<String>,
    #[serde(default)]
    pub proxy_controller_address: Option<String>,
    #[serde(default)]
    pub proxy_controller_secret_account: Option<String>,
    #[serde(default)]
    pub proxy_network_service: Option<String>,
}

/// Validate and canonicalize a directory path for project import (symlinks rejected).
pub fn validate_import_project_path(path: &Path) -> Result<PathBuf, String> {
    if !path.exists() {
        return Err("path does not exist".into());
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };

    // Check every existing path component before canonicalizing. Checking only the final
    // component would allow `symlink-to-outside/project` to escape the import boundary.
    let mut current = PathBuf::new();
    for component in absolute.components() {
        match component {
            std::path::Component::Prefix(prefix) => current.push(prefix.as_os_str()),
            std::path::Component::RootDir => current.push(component.as_os_str()),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                current.pop();
            }
            std::path::Component::Normal(name) => {
                current.push(name);
                let meta = fs::symlink_metadata(&current).map_err(|e| e.to_string())?;
                if meta.file_type().is_symlink() && !is_macos_system_alias(&current) {
                    return Err("symlink not allowed".into());
                }
            }
        }
    }

    let meta = fs::symlink_metadata(&absolute).map_err(|e| e.to_string())?;
    if meta.file_type().is_symlink() && !is_macos_system_alias(&absolute) {
        return Err("symlink not allowed".into());
    }
    if !meta.is_dir() {
        return Err("path is not a directory".into());
    }
    absolute.canonicalize().map_err(|e| e.to_string())
}

/// macOS exposes `/tmp`, `/var`, and `/etc` as stable aliases into `/private`.
/// They are OS path aliases, not user-controlled project symlinks; allow them while
/// still rejecting every symlink below those roots.
#[cfg(unix)]
fn is_macos_system_alias(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if !matches!(name, "tmp" | "var" | "etc") {
        return false;
    }
    let expected = Path::new("/private").join(name);
    path.is_absolute() && path.canonicalize().ok().as_deref() == Some(expected.as_path())
}

#[cfg(not(unix))]
fn is_macos_system_alias(_path: &Path) -> bool {
    false
}

impl LumaSettings {
    pub fn import_project_path(&mut self, path: &Path) -> Result<(), String> {
        let canon = validate_import_project_path(path)?;
        let canon_str = canon.display().to_string();
        if self.imported_projects.iter().any(|p| p.path == canon_str) {
            return Err("project already imported".into());
        }
        let name = canon
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("project")
            .to_string();
        self.imported_projects.push(ImportedProject {
            path: canon_str,
            name: Some(name),
        });
        Ok(())
    }

    pub fn remove_imported_project(&mut self, key: &str) -> Result<(), String> {
        let key_path = Path::new(key);
        let path_like = key_path.is_absolute()
            || key_path.components().count() > 1
            || key.starts_with('.')
            || key.contains(std::path::MAIN_SEPARATOR);
        let canonical_key = if path_like && key_path.exists() {
            Some(
                validate_import_project_path(key_path)?
                    .display()
                    .to_string(),
            )
        } else {
            None
        };

        let path_matches: Vec<usize> = self
            .imported_projects
            .iter()
            .enumerate()
            .filter_map(|(index, project)| {
                let stored_canonical = Path::new(&project.path)
                    .canonicalize()
                    .ok()
                    .map(|path| path.display().to_string());
                if project.path == key || canonical_key.as_deref() == stored_canonical.as_deref() {
                    Some(index)
                } else {
                    None
                }
            })
            .collect();

        let matches = if path_matches.is_empty() {
            self.imported_projects
                .iter()
                .enumerate()
                .filter_map(|(index, project)| {
                    let base = Path::new(&project.path)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    (project.name.as_deref() == Some(key) || base == key).then_some(index)
                })
                .collect::<Vec<_>>()
        } else {
            path_matches
        };

        if matches.is_empty() {
            return Err(format!("no imported project matching \"{key}\""));
        }
        if matches.len() > 1 {
            return Err(format!(
                "ambiguous imported project \"{key}\"; remove it by full path"
            ));
        }
        self.imported_projects.remove(matches[0]);
        Ok(())
    }

    /// Apply a JSON settings patch (shared by engine `UpdateSettings` and CLI `config set`).
    ///
    /// Unknown keys are ignored. Empty strings clear optional path/string fields.
    /// `enabled_modules` keys are sticky: this only upserts flags, never deletes ids.
    pub fn apply_settings_patch(&mut self, patch: &serde_json::Value) -> Result<(), String> {
        if let Some(obj) = patch.get("enabled_modules").and_then(|v| v.as_object()) {
            for (id, value) in obj {
                if let Some(enabled) = value.as_bool() {
                    self.enabled_modules.insert(id.clone(), enabled);
                }
            }
        }
        if let Some(root) = patch.get("notes_root") {
            if root.is_null() {
                self.notes_root = None;
            } else if let Some(s) = root.as_str() {
                self.notes_root = if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                };
            }
        }
        if let Some(root) = patch.get("records_root") {
            if root.is_null() {
                self.records_root = None;
            } else if let Some(s) = root.as_str() {
                self.records_root = if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                };
            }
        }
        if let Some(roots) = patch.get("projects_roots").and_then(|v| v.as_array()) {
            self.projects_roots = roots
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect();
        }
        if let Some(patterns) = patch
            .get("notes_exclude_patterns")
            .and_then(|v| v.as_array())
        {
            self.notes_exclude_patterns = patterns
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .filter(|p| !p.is_empty())
                .collect();
        }
        if let Some(days) = patch
            .get("clipboard_retention_days")
            .and_then(|v| v.as_u64())
        {
            self.clipboard_retention_days = days as u32;
        }
        if let Some(secs) = patch.get("secrets_idle_lock_secs").and_then(|v| v.as_u64()) {
            self.secrets_idle_lock_secs = secs as u32;
        }
        if let Some(max) = patch.get("hub_windows_max").and_then(|v| v.as_u64()) {
            self.hub_windows_max = (max as u32).clamp(5, 50);
        }
        apply_optional_string_field(
            patch,
            "proxy_controller_unix_socket",
            &mut self.proxy_controller_unix_socket,
        );
        apply_optional_string_field(
            patch,
            "proxy_controller_address",
            &mut self.proxy_controller_address,
        );
        apply_optional_string_field(
            patch,
            "proxy_controller_secret_account",
            &mut self.proxy_controller_secret_account,
        );
        apply_optional_string_field(
            patch,
            "proxy_network_service",
            &mut self.proxy_network_service,
        );

        for path in patch_string_list(patch, "import_project", "import_projects") {
            self.import_project_path(std::path::Path::new(&path))?;
        }
        for name in patch_string_list(patch, "remove_project", "remove_projects") {
            self.remove_imported_project(&name)?;
        }
        Ok(())
    }
}

fn apply_optional_string_field(patch: &serde_json::Value, key: &str, field: &mut Option<String>) {
    let Some(value) = patch.get(key) else {
        return;
    };
    if value.is_null() {
        *field = None;
    } else if let Some(s) = value.as_str() {
        *field = if s.is_empty() {
            None
        } else {
            Some(s.to_string())
        };
    }
}

fn patch_string_list(patch: &serde_json::Value, singular: &str, plural: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(s) = patch.get(singular).and_then(|v| v.as_str()) {
        if !s.is_empty() {
            out.push(s.to_string());
        }
    }
    if let Some(arr) = patch.get(plural).and_then(|v| v.as_array()) {
        for v in arr {
            if let Some(s) = v.as_str() {
                if !s.is_empty() {
                    out.push(s.to_string());
                }
            }
        }
    }
    out
}

fn default_secrets_idle_lock_secs() -> u32 {
    300
}

fn default_hub_windows_max() -> u32 {
    15
}

impl Default for LumaSettings {
    fn default() -> Self {
        let mut enabled_modules = BTreeMap::new();
        enabled_modules.insert("luma.apps".into(), true);
        enabled_modules.insert("luma.windows".into(), true);
        enabled_modules.insert("luma.proxy".into(), true);
        enabled_modules.insert("luma.clipboard".into(), true);
        enabled_modules.insert("luma.notes".into(), true);
        enabled_modules.insert("luma.records".into(), true);
        enabled_modules.insert("luma.secrets".into(), false);
        enabled_modules.insert("luma.fake".into(), false);
        Self {
            schema_version: 1,
            settings_version: 1,
            enabled_modules,
            notes_root: None,
            projects_roots: Vec::new(),
            imported_projects: Vec::new(),
            notes_exclude_patterns: Vec::new(),
            records_root: None,
            clipboard_retention_days: 30,
            secrets_idle_lock_secs: default_secrets_idle_lock_secs(),
            hub_windows_max: default_hub_windows_max(),
            proxy_controller_unix_socket: None,
            proxy_controller_address: None,
            proxy_controller_secret_account: None,
            proxy_network_service: None,
        }
    }
}

pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn luma_next_default() -> Result<Self, ConfigError> {
        ensure_luma_next_dirs()?;
        Ok(Self {
            path: luma_next_support_dir()?.join("settings.toml"),
        })
    }

    pub fn with_path(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load_or_default(&self) -> Result<LumaSettings, ConfigError> {
        self.with_settings_lock(|_| self.load_or_default_unlocked())
    }

    fn load_or_default_unlocked(&self) -> Result<LumaSettings, ConfigError> {
        let marker = self.corrupt_marker_path();
        if marker.exists() {
            return Err(ConfigError::Corrupt(marker));
        }
        if !self.path.exists() {
            let settings = LumaSettings::default();
            self.save_unlocked(&settings)?;
            return Ok(settings);
        }
        let raw = fs::read_to_string(&self.path)?;
        match toml::from_str::<LumaSettings>(&raw) {
            Ok(s) => Ok(s),
            Err(_) => {
                let quarantine = self.path.with_extension(format!(
                    "corrupt-{}.toml",
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0)
                ));
                fs::rename(&self.path, &quarantine)?;
                // Block silent re-init that would re-enable modules.
                fs::write(
                    &marker,
                    format!(
                        "quarantined={}\nremediation=inspect quarantined file then delete this marker and run `luma config set` or restore settings.toml\n",
                        quarantine.display()
                    ),
                )?;
                Err(ConfigError::Corrupt(quarantine))
            }
        }
    }

    fn corrupt_marker_path(&self) -> PathBuf {
        self.path.with_extension("toml.corrupt-marker")
    }

    /// Clear the corrupt marker after an operator restores settings (tests / recovery).
    pub fn clear_corrupt_marker(&self) -> Result<(), ConfigError> {
        let marker = self.corrupt_marker_path();
        if marker.exists() {
            fs::remove_file(marker)?;
        }
        Ok(())
    }

    pub fn save(&self, settings: &LumaSettings) -> Result<(), ConfigError> {
        self.with_settings_lock(|_| self.save_unlocked(settings))
    }

    fn save_unlocked(&self, settings: &LumaSettings) -> Result<(), ConfigError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let body = toml::to_string_pretty(settings)?;
        let tmp = self
            .path
            .with_extension(format!("toml.tmp.{}", std::process::id()));
        fs::write(&tmp, body)?;
        {
            let file = OpenOptions::new().write(true).open(&tmp)?;
            file.sync_all()?;
        }
        fs::rename(&tmp, &self.path)?;
        if let Some(parent) = self.path.parent() {
            if let Ok(dir) = File::open(parent) {
                let _ = dir.sync_all();
            }
        }
        Ok(())
    }

    pub fn update_cas(
        &self,
        expected_version: u64,
        mut patch: LumaSettings,
    ) -> Result<LumaSettings, ConfigError> {
        self.with_settings_lock(|_| {
            let current = self.load_or_default_unlocked()?;
            if current.settings_version != expected_version {
                return Err(ConfigError::VersionConflict {
                    expected: expected_version,
                    found: current.settings_version,
                });
            }
            patch.settings_version = current.settings_version + 1;
            patch.schema_version = current.schema_version;
            self.save_unlocked(&patch)?;
            Ok(patch)
        })
    }

    /// Read settings under lock, apply `mutate`, then CAS-save (single critical section).
    pub fn mutate_settings(
        &self,
        expected_version: Option<u64>,
        mutate: impl FnOnce(&mut LumaSettings),
    ) -> Result<LumaSettings, ConfigError> {
        self.try_mutate_settings(expected_version, |next| {
            mutate(next);
            Ok(())
        })
    }

    /// Read settings under lock, apply a fallible mutation, then CAS-save atomically.
    pub fn try_mutate_settings(
        &self,
        expected_version: Option<u64>,
        mutate: impl FnOnce(&mut LumaSettings) -> Result<(), String>,
    ) -> Result<LumaSettings, ConfigError> {
        self.with_settings_lock(|_| {
            let current = self.load_or_default_unlocked()?;
            let expected = expected_version.unwrap_or(current.settings_version);
            if current.settings_version != expected {
                return Err(ConfigError::VersionConflict {
                    expected,
                    found: current.settings_version,
                });
            }
            let mut next = current;
            mutate(&mut next).map_err(ConfigError::Mutation)?;
            next.settings_version = expected + 1;
            self.save_unlocked(&next)?;
            Ok(next)
        })
    }

    fn with_settings_lock<T>(
        &self,
        f: impl FnOnce(&Path) -> Result<T, ConfigError>,
    ) -> Result<T, ConfigError> {
        let _lock = SettingsLock::acquire(&self.path)?;
        f(&self.path)
    }
}

#[cfg(unix)]
fn flock_exclusive(file: &File) -> Result<(), ConfigError> {
    use std::os::unix::io::AsRawFd;
    extern "C" {
        fn flock(fd: std::os::fd::RawFd, operation: i32) -> i32;
    }
    const LOCK_EX: i32 = 0x2;
    let ret = unsafe { flock(file.as_raw_fd(), LOCK_EX) };
    if ret == 0 {
        Ok(())
    } else {
        Err(ConfigError::Io(std::io::Error::last_os_error()))
    }
}

#[cfg(not(unix))]
fn flock_exclusive(_file: &File) -> Result<(), ConfigError> {
    Ok(())
}

struct SettingsLock {
    _file: File,
    _data: Option<File>,
}

impl SettingsLock {
    fn acquire(settings_path: &Path) -> Result<Self, ConfigError> {
        let lock_path = settings_path.with_extension("toml.lock");
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent)?;
        }
        // Blocking flock: wait for any other Luma process holding this lock. Keep this
        // pathname after release: unlinking it while another process waits on the old
        // inode would let a third process create and lock a new, independent inode.
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&lock_path)?;
        flock_exclusive(&file)?;
        let _ = writeln!(file, "pid={}", std::process::id());
        let _ = file.sync_all();
        let data = if settings_path.exists() {
            let df = OpenOptions::new()
                .read(true)
                .write(true)
                .open(settings_path)?;
            flock_exclusive(&df)?;
            Some(df)
        } else {
            None
        };
        Ok(Self {
            _file: file,
            _data: data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use tempfile::tempdir;

    /// Serialize lock/CAS tests that deliberately coordinate competing file descriptors.
    static CONFIG_FILE_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn config_test_lock() -> std::sync::MutexGuard<'static, ()> {
        CONFIG_FILE_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[test]
    fn round_trip_and_cas() {
        let dir = tempdir().unwrap();
        let store = ConfigStore::with_path(dir.path().join("settings.toml"));
        let s = store.load_or_default().unwrap();
        assert_eq!(s.settings_version, 1);
        let mut next = s.clone();
        next.notes_root = Some("/tmp/notes".into());
        let saved = store.update_cas(1, next).unwrap();
        assert_eq!(saved.settings_version, 2);
        assert!(store.update_cas(1, saved.clone()).is_err());
    }

    #[test]
    fn corrupt_is_quarantined() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        fs::write(&path, "not = toml [[[").unwrap();
        let store = ConfigStore::with_path(path);
        let err = store.load_or_default().unwrap_err();
        assert!(matches!(err, ConfigError::Corrupt(_)));
    }

    #[test]
    fn stale_lock_metadata_does_not_block_and_lock_file_is_retained() {
        let _guard = config_test_lock();
        let dir = tempdir().unwrap();
        let settings = dir.path().join("settings.toml");
        let lock = settings.with_extension("toml.lock");
        fs::write(&lock, "pid=999999999\n").unwrap();
        let store = ConfigStore::with_path(settings);
        let loaded = store.load_or_default().unwrap();
        assert_eq!(loaded.settings_version, 1);
        // `flock` is released when a process exits; the PID text is only diagnostic.
        // The stable lock pathname must remain so all contenders lock one inode.
        assert!(lock.exists());
        assert_eq!(store.load_or_default().unwrap().settings_version, 1);
        assert!(lock.exists());
    }

    #[cfg(unix)]
    fn flock_exclusive_nonblocking(file: &File) -> std::io::Result<()> {
        use std::os::unix::io::AsRawFd;

        extern "C" {
            fn flock(fd: std::os::fd::RawFd, operation: i32) -> i32;
        }

        const LOCK_EX: i32 = 0x2;
        const LOCK_NB: i32 = 0x4;
        if unsafe { flock(file.as_raw_fd(), LOCK_EX | LOCK_NB) } == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }

    #[cfg(unix)]
    #[test]
    fn lock_inode_stays_stable_across_waiter_and_third_contender() {
        use std::os::unix::fs::MetadataExt;
        use std::sync::mpsc;
        use std::time::Duration;

        let _guard = config_test_lock();
        let dir = tempdir().unwrap();
        let settings = dir.path().join("settings.toml");
        let lock_path = settings.with_extension("toml.lock");

        let first = SettingsLock::acquire(&settings).unwrap();
        let initial_inode = fs::metadata(&lock_path).unwrap().ino();

        // Open the waiter before releasing `first`, so it holds a descriptor for the
        // original inode while waiting on its flock.
        let (waiter_opened_tx, waiter_opened_rx) = mpsc::channel();
        let (waiter_acquired_tx, waiter_acquired_rx) = mpsc::channel();
        let (release_waiter_tx, release_waiter_rx) = mpsc::channel();
        let waiter_path = lock_path.clone();
        let waiter = thread::spawn(move || {
            let file = OpenOptions::new().write(true).open(&waiter_path).unwrap();
            waiter_opened_tx.send(()).unwrap();
            flock_exclusive(&file).unwrap();
            waiter_acquired_tx.send(()).unwrap();
            let _ = release_waiter_rx.recv();
        });
        waiter_opened_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("waiter should open the original lock inode");

        drop(first);
        waiter_acquired_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("waiter should acquire after the first holder releases");

        let current_inode = fs::metadata(&lock_path)
            .expect("lock pathname must remain while a waiter holds it")
            .ino();
        assert_eq!(
            current_inode, initial_inode,
            "lock inode changed during handoff"
        );

        // A third contender must open that same inode and observe the waiter's flock.
        // The old Drop-based unlink allowed this open to create a second inode instead.
        let third = OpenOptions::new().write(true).open(&lock_path).unwrap();
        let err = flock_exclusive_nonblocking(&third)
            .expect_err("third contender must not acquire a parallel lock inode");
        assert_eq!(err.kind(), std::io::ErrorKind::WouldBlock);
        drop(third);

        release_waiter_tx.send(()).unwrap();
        waiter.join().unwrap();

        drop(SettingsLock::acquire(&settings).unwrap());
        assert!(lock_path.exists());
    }

    #[test]
    fn concurrent_cas_one_succeeds() {
        let _guard = config_test_lock();
        use std::sync::Barrier;
        let dir = tempdir().unwrap();
        let store = ConfigStore::with_path(dir.path().join("settings.toml"));
        let base = store.load_or_default().unwrap();
        assert_eq!(base.settings_version, 1);
        let store = Arc::new(store);
        let barrier = Arc::new(Barrier::new(2));
        let handles: Vec<_> = (0..2u8)
            .map(|i| {
                let store = Arc::clone(&store);
                let barrier = Arc::clone(&barrier);
                let mut next = base.clone();
                next.notes_root = Some(format!("/tmp/{i}"));
                thread::spawn(move || {
                    barrier.wait();
                    store.update_cas(1, next)
                })
            })
            .collect();
        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        let ok = results.iter().filter(|r| r.is_ok()).count();
        let conflict = results
            .iter()
            .filter(|r| matches!(r, Err(ConfigError::VersionConflict { .. })))
            .count();
        let lock_timeout = results
            .iter()
            .filter(|r| matches!(r, Err(ConfigError::LockTimeout)))
            .count();
        assert_eq!(ok, 1, "results: {results:?} (lock_timeouts={lock_timeout})");
        assert!(
            ok + conflict + lock_timeout == 2,
            "each attempt must finish: {results:?}"
        );
        let final_settings = store.load_or_default().unwrap();
        assert_eq!(final_settings.settings_version, 2);
    }

    #[test]
    fn apply_settings_patch_updates_modules_and_roots() {
        let mut settings = LumaSettings::default();
        settings
            .apply_settings_patch(&serde_json::json!({
                "enabled_modules": { "luma.fake": true },
                "notes_root": "/tmp/notes",
                "hub_windows_max": 99,
                "notes_exclude_patterns": ["private/*", ""],
            }))
            .unwrap();
        assert_eq!(settings.enabled_modules.get("luma.fake"), Some(&true));
        assert_eq!(settings.notes_root.as_deref(), Some("/tmp/notes"));
        assert_eq!(settings.hub_windows_max, 50);
        assert_eq!(
            settings.notes_exclude_patterns,
            vec!["private/*".to_string()]
        );
        // Sticky: unrelated module keys remain.
        assert_eq!(settings.enabled_modules.get("luma.apps"), Some(&true));
    }

    #[test]
    fn import_project_persists_and_rejects_duplicate() {
        let dir = tempdir().unwrap();
        let project = dir.path().join("MyApp");
        fs::create_dir(&project).unwrap();
        let store = ConfigStore::with_path(dir.path().join("settings.toml"));
        let mut settings = store.load_or_default().unwrap();
        settings.import_project_path(&project).unwrap();
        store.save(&settings).unwrap();
        let loaded = store.load_or_default().unwrap();
        assert_eq!(loaded.imported_projects.len(), 1);
        assert!(loaded.imported_projects[0]
            .name
            .as_deref()
            .is_some_and(|n| n == "MyApp"));
        let err = loaded.clone().import_project_path(&project).unwrap_err();
        assert!(err.contains("already imported"), "{err}");
    }

    #[test]
    fn remove_imported_project_keeps_directory() {
        let dir = tempdir().unwrap();
        let project = dir.path().join("KeepMe");
        fs::create_dir(&project).unwrap();
        fs::write(project.join("marker.txt"), "x").unwrap();
        let store = ConfigStore::with_path(dir.path().join("settings.toml"));
        let mut settings = store.load_or_default().unwrap();
        settings.import_project_path(&project).unwrap();
        settings.remove_imported_project("KeepMe").unwrap();
        store.save(&settings).unwrap();
        assert!(project.join("marker.txt").exists());
        assert!(store
            .load_or_default()
            .unwrap()
            .imported_projects
            .is_empty());
    }

    #[test]
    fn import_rejects_symlink() {
        let dir = tempdir().unwrap();
        let real = dir.path().join("real");
        fs::create_dir(&real).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let link = dir.path().join("link");
            symlink(&real, &link).unwrap();
            let mut settings = LumaSettings::default();
            let err = settings.import_project_path(&link).unwrap_err();
            assert!(err.contains("symlink"), "{err}");
        }
    }

    #[test]
    fn import_rejects_symlink_ancestor() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let real_parent = outside.path().join("parent");
        fs::create_dir(&real_parent).unwrap();
        let project = real_parent.join("project");
        fs::create_dir(&project).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let link = dir.path().join("link");
            symlink(&real_parent, &link).unwrap();
            let mut settings = LumaSettings::default();
            let err = settings
                .import_project_path(&link.join("project"))
                .unwrap_err();
            assert!(err.contains("symlink"), "{err}");
        }
    }

    #[test]
    fn legacy_settings_without_imported_projects_load_empty() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        let raw = toml::to_string_pretty(&LumaSettings::default())
            .unwrap()
            .replace("imported_projects = []\n", "");
        fs::write(&path, raw).unwrap();
        let loaded = ConfigStore::with_path(path).load_or_default().unwrap();
        assert!(loaded.imported_projects.is_empty());
    }

    #[test]
    fn remove_by_path_is_canonical_and_names_must_be_unambiguous() {
        let dir = tempdir().unwrap();
        let first_parent = dir.path().join("one");
        let second_parent = dir.path().join("two");
        fs::create_dir_all(&first_parent).unwrap();
        fs::create_dir_all(&second_parent).unwrap();
        let first = first_parent.join("same");
        let second = second_parent.join("same");
        fs::create_dir(&first).unwrap();
        fs::create_dir(&second).unwrap();
        let mut settings = LumaSettings::default();
        settings.import_project_path(&first).unwrap();
        settings.import_project_path(&second).unwrap();
        let err = settings.remove_imported_project("same").unwrap_err();
        assert!(err.contains("ambiguous"), "{err}");
        settings
            .remove_imported_project(&first.display().to_string())
            .unwrap();
        assert_eq!(settings.imported_projects.len(), 1);
        assert!(settings.imported_projects[0].path.ends_with("/two/same"));
    }
}
