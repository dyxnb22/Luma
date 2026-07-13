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
        let rd = std::fs::read_dir(root).map_err(|e| e.to_string())?;
        for entry in rd.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("app") {
                continue;
            }
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
        }
    }
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps.dedup_by(|a, b| a.path == b.path);
    Ok(apps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

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
