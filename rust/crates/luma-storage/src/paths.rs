use std::fs;
use std::path::PathBuf;
use thiserror::Error;

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

/// Diagnostic exports under Application Support (`LumaNext/diagnostics`).
pub fn luma_next_diagnostics_dir() -> Result<PathBuf, PathsError> {
    Ok(luma_next_support_dir()?.join("diagnostics"))
}

pub fn ensure_luma_next_dirs() -> Result<(), PathsError> {
    fs::create_dir_all(luma_next_support_dir()?)?;
    fs::create_dir_all(luma_next_logs_dir()?)?;
    fs::create_dir_all(luma_next_diagnostics_dir()?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_use_lumanext_not_luma() {
        // Clear overrides for this assertion when unset.
        if std::env::var_os("LUMA_NEXT_SUPPORT_DIR").is_none() {
            let support = luma_next_support_dir().unwrap();
            assert!(support.ends_with("LumaNext"));
        }
        if std::env::var_os("LUMA_NEXT_LOGS_DIR").is_none() {
            let logs = luma_next_logs_dir().unwrap();
            assert!(logs.ends_with("LumaNext"));
        }
    }
}
