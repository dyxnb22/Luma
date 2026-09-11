use std::fs;
use std::path::PathBuf;
use thiserror::Error;

#[cfg(test)]
use std::ffi::OsString;
#[cfg(test)]
use std::path::Path;
#[cfg(test)]
use std::sync::{Mutex, MutexGuard};

#[derive(Debug, Error)]
pub enum PathsError {
    #[error("home directory unavailable")]
    NoHome,
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// Active application-support root (`LumaNext`).
/// Override with `LUMA_NEXT_SUPPORT_DIR` for isolated test roots.
pub fn luma_next_support_dir() -> Result<PathBuf, PathsError> {
    if let Ok(p) = std::env::var("LUMA_NEXT_SUPPORT_DIR") {
        return Ok(PathBuf::from(p));
    }
    let home = dirs::home_dir().ok_or(PathsError::NoHome)?;
    Ok(home
        .join("Library")
        .join("Application Support")
        .join("LumaNext"))
}

pub fn luma_next_logs_dir() -> Result<PathBuf, PathsError> {
    if let Ok(p) = std::env::var("LUMA_NEXT_LOGS_DIR") {
        return Ok(PathBuf::from(p));
    }
    let home = dirs::home_dir().ok_or(PathsError::NoHome)?;
    Ok(home.join("Library").join("Logs").join("LumaNext"))
}

pub fn ensure_luma_next_dirs() -> Result<(), PathsError> {
    fs::create_dir_all(luma_next_support_dir()?)?;
    fs::create_dir_all(luma_next_logs_dir()?)?;
    Ok(())
}

#[cfg(test)]
static LUMA_NEXT_PATH_ENV_LOCK: Mutex<()> = Mutex::new(());

/// Serializes tests which temporarily override the process-wide LumaNext paths.
///
/// The guard retains the lock for its lifetime and restores both variables exactly
/// as they were, including non-Unicode values, on drop. Tests that inspect the
/// resolved default paths should use [`Self::hold`] so they cannot observe a
/// temporary override from another test.
#[cfg(test)]
pub(crate) struct LumaNextTestEnvGuard {
    _lock: MutexGuard<'static, ()>,
    previous: Option<PreviousLumaNextPathEnv>,
}

#[cfg(test)]
struct PreviousLumaNextPathEnv {
    support_dir: Option<OsString>,
    logs_dir: Option<OsString>,
}

#[cfg(test)]
impl LumaNextTestEnvGuard {
    /// Holds the shared environment lock without changing either path override.
    pub(crate) fn hold() -> Self {
        Self {
            _lock: lock_luma_next_path_env(),
            previous: None,
        }
    }

    /// Overrides both LumaNext roots for one test and restores the prior values
    /// when the guard is dropped.
    pub(crate) fn override_paths(support_dir: &Path, logs_dir: &Path) -> Self {
        let lock = lock_luma_next_path_env();
        let previous = PreviousLumaNextPathEnv {
            support_dir: std::env::var_os("LUMA_NEXT_SUPPORT_DIR"),
            logs_dir: std::env::var_os("LUMA_NEXT_LOGS_DIR"),
        };
        std::env::set_var("LUMA_NEXT_SUPPORT_DIR", support_dir);
        std::env::set_var("LUMA_NEXT_LOGS_DIR", logs_dir);
        Self {
            _lock: lock,
            previous: Some(previous),
        }
    }
}

#[cfg(test)]
impl Drop for LumaNextTestEnvGuard {
    fn drop(&mut self) {
        let Some(previous) = self.previous.take() else {
            return;
        };

        restore_env_var("LUMA_NEXT_SUPPORT_DIR", previous.support_dir);
        restore_env_var("LUMA_NEXT_LOGS_DIR", previous.logs_dir);
    }
}

#[cfg(test)]
fn lock_luma_next_path_env() -> MutexGuard<'static, ()> {
    LUMA_NEXT_PATH_ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
fn restore_env_var(name: &str, previous: Option<OsString>) {
    match previous {
        Some(value) => std::env::set_var(name, value),
        None => std::env::remove_var(name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn paths_use_lumanext_not_luma() {
        let _env = LumaNextTestEnvGuard::hold();

        if std::env::var_os("LUMA_NEXT_SUPPORT_DIR").is_none() {
            let support = luma_next_support_dir().unwrap();
            assert!(support.ends_with("LumaNext"));
        }
        if std::env::var_os("LUMA_NEXT_LOGS_DIR").is_none() {
            let logs = luma_next_logs_dir().unwrap();
            assert!(logs.ends_with("LumaNext"));
        }
    }

    #[test]
    fn test_path_override_restores_the_previous_values_and_holds_the_lock() {
        let dir = tempdir().unwrap();
        let support = dir.path().join("support");
        let logs = dir.path().join("logs");
        let env = LumaNextTestEnvGuard::override_paths(&support, &logs);
        let previous = env.previous.as_ref().unwrap();
        let previous_support = previous.support_dir.clone();
        let previous_logs = previous.logs_dir.clone();

        assert_eq!(luma_next_support_dir().unwrap(), support);
        assert_eq!(luma_next_logs_dir().unwrap(), logs);
        assert!(matches!(
            LUMA_NEXT_PATH_ENV_LOCK.try_lock(),
            Err(std::sync::TryLockError::WouldBlock)
        ));

        drop(env);

        assert_eq!(std::env::var_os("LUMA_NEXT_SUPPORT_DIR"), previous_support);
        assert_eq!(std::env::var_os("LUMA_NEXT_LOGS_DIR"), previous_logs);
        assert!(LUMA_NEXT_PATH_ENV_LOCK.try_lock().is_ok());
    }
}
