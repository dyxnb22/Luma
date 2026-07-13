use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppEntry {
    pub name: String,
    pub path: PathBuf,
    pub bundle_id: Option<String>,
}

#[derive(Debug, Error)]
pub enum AppLaunchError {
    #[error("app not found: {0}")]
    NotFound(String),
    #[error("launch failed: {0}")]
    LaunchFailed(String),
}

#[async_trait]
pub trait AppsCatalogPort: Send + Sync {
    async fn list_installed(&self) -> Result<Vec<AppEntry>, String>;
    async fn launch(&self, path: &Path) -> Result<(), AppLaunchError>;
    async fn reveal(&self, path: &Path) -> Result<(), AppLaunchError>;
}
