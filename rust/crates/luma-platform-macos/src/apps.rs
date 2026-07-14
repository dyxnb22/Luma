use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::debug;

pub use luma_application::{AppEntry, AppLaunchError, AppsCatalogPort as AppsCatalog};

/// Scans standard Applications directories. Search path never re-scans; warmup caches.
pub struct FilesystemAppsCatalog {
    roots: Vec<PathBuf>,
}

impl FilesystemAppsCatalog {
    pub fn system_default() -> Self {
        let mut roots = vec![
            PathBuf::from("/Applications"),
            PathBuf::from("/System/Applications"),
        ];
        if let Some(home) = std::env::var_os("HOME") {
            roots.push(PathBuf::from(home).join("Applications"));
        }
        Self { roots }
    }

    pub fn with_roots(roots: Vec<PathBuf>) -> Self {
        Self { roots }
    }
}

#[async_trait]
impl AppsCatalog for FilesystemAppsCatalog {
    async fn list_installed(&self) -> Result<Vec<AppEntry>, String> {
        let roots = self.roots.clone();
        tokio::task::spawn_blocking(move || scan_roots(&roots))
            .await
            .map_err(|e| e.to_string())?
    }

    async fn launch(&self, path: &Path) -> Result<(), AppLaunchError> {
        let status = Command::new("open")
            .arg(path)
            .status()
            .await
            .map_err(|e| AppLaunchError::LaunchFailed(e.to_string()))?;
        if status.success() {
            Ok(())
        } else {
            Err(AppLaunchError::LaunchFailed(format!(
                "open exited {status}"
            )))
        }
    }

    async fn reveal(&self, path: &Path) -> Result<(), AppLaunchError> {
        let status = Command::new("open")
            .args(["-R"])
            .arg(path)
            .status()
            .await
            .map_err(|e| AppLaunchError::LaunchFailed(e.to_string()))?;
        if status.success() {
            Ok(())
        } else {
            Err(AppLaunchError::LaunchFailed(format!(
                "open -R exited {status}"
            )))
        }
    }
}

fn scan_roots(roots: &[PathBuf]) -> Result<Vec<AppEntry>, String> {
    let mut apps = Vec::new();
    for root in roots {
        if !root.exists() {
            continue;
        }
        scan_dir(root, 0, &mut apps)?;
    }
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps.dedup_by(|a, b| a.path == b.path);
    Ok(apps)
}

const MAX_SCAN_DEPTH: usize = 4;

fn scan_dir(dir: &Path, depth: usize, apps: &mut Vec<AppEntry>) -> Result<(), String> {
    if depth > MAX_SCAN_DEPTH {
        return Ok(());
    }
    let rd = std::fs::read_dir(dir).map_err(|e| e.to_string())?;
    for entry in rd.flatten() {
        let path = entry.path();
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.file_type().is_symlink() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) == Some("app") {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown")
                .to_string();
            debug!(%name, path = %path.display(), "indexed app");
            apps.push(AppEntry {
                name,
                path,
                bundle_id: None,
            });
            continue;
        }
        if meta.is_dir() {
            scan_dir(&path, depth + 1, apps)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn scans_nested_fixture_app() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("Setapp").join("Nested")).unwrap();
        fs::create_dir(dir.path().join("Setapp").join("Nested").join("Bear.app")).unwrap();
        let catalog = FilesystemAppsCatalog::with_roots(vec![dir.path().to_path_buf()]);
        let apps = catalog.list_installed().await.unwrap();
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "Bear");
    }

    #[tokio::test]
    async fn scans_fixture_apps() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("Safari.app")).unwrap();
        fs::create_dir(dir.path().join("Mail.app")).unwrap();
        let catalog = FilesystemAppsCatalog::with_roots(vec![dir.path().to_path_buf()]);
        let apps = catalog.list_installed().await.unwrap();
        assert_eq!(apps.len(), 2);
        assert!(apps.iter().any(|a| a.name == "Safari"));
    }
}
