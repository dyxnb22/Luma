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
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LumaSettings {
    pub schema_version: u32,
    pub settings_version: u64,
    pub enabled_modules: BTreeMap<String, bool>,
    pub notes_root: Option<String>,
    #[serde(default)]
    pub projects_roots: Vec<String>,
    /// Glob patterns relative to notes_root (e.g. `private/*`).
    #[serde(default)]
    pub notes_exclude_patterns: Vec<String>,
    pub clipboard_retention_days: u32,
}

impl Default for LumaSettings {
    fn default() -> Self {
        let mut enabled_modules = BTreeMap::new();
        enabled_modules.insert("luma.apps".into(), true);
        enabled_modules.insert("luma.windows".into(), true);
        enabled_modules.insert("luma.clipboard".into(), true);
        enabled_modules.insert("luma.notes".into(), true);
        enabled_modules.insert("luma.secrets".into(), false);
        enabled_modules.insert("luma.fake".into(), false);
        Self {
            schema_version: 1,
            settings_version: 1,
            enabled_modules,
            notes_root: None,
            projects_roots: Vec::new(),
            notes_exclude_patterns: Vec::new(),
            clipboard_retention_days: 30,
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
            mutate(&mut next);
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
    path: PathBuf,
    _file: File,
    _data: Option<File>,
}

impl SettingsLock {
    fn acquire(settings_path: &Path) -> Result<Self, ConfigError> {
        let lock_path = settings_path.with_extension("toml.lock");
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent)?;
        }
        // Blocking flock: wait for any other Luma process holding this lock.
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&lock_path)?;
        flock_exclusive(&file)?;
        let _ = writeln!(file, "pid={}", std::process::id());
        let _ = file.sync_all();
        let data = if settings_path.exists() {
            let df = OpenOptions::new().read(true).write(true).open(settings_path)?;
            flock_exclusive(&df)?;
            Some(df)
        } else {
            None
        };
        Ok(Self {
            path: lock_path,
            _file: file,
            _data: data,
        })
    }
}

impl Drop for SettingsLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use tempfile::tempdir;

    /// Config lock/CAS tests share PID-based lock cleanup; serialize to avoid cross-test races.
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
    fn stale_lock_from_dead_pid_is_removed() {
        let _guard = config_test_lock();
        let dir = tempdir().unwrap();
        let settings = dir.path().join("settings.toml");
        let lock = settings.with_extension("toml.lock");
        fs::write(&lock, "pid=999999999\n").unwrap();
        let store = ConfigStore::with_path(settings);
        let loaded = store.load_or_default().unwrap();
        assert_eq!(loaded.settings_version, 1);
        assert!(!lock.exists());
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
}
