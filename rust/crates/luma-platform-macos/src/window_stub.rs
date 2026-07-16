//! Non-macOS stub so the crate can compile for unit tests on Linux hosts.

use async_trait::async_trait;
use luma_application::{WindowCatalogPort, WindowEntry, WindowError};

pub struct MacWindowCatalog;

impl Default for MacWindowCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl MacWindowCatalog {
    pub fn new() -> Self {
        Self
    }

    /// Sync snapshot for composition root (before async runtime is attached).
    pub fn snapshot_previous_frontmost_app_sync(&self) -> Result<Option<String>, WindowError> {
        Err(unavailable())
    }
}

fn unavailable() -> WindowError {
    WindowError::Unavailable("macOS window APIs are unavailable on this platform".into())
}

#[async_trait]
impl WindowCatalogPort for MacWindowCatalog {
    async fn snapshot_previous_frontmost_app(&self) -> Result<Option<String>, WindowError> {
        Err(unavailable())
    }

    async fn previous_frontmost_app(&self) -> Option<String> {
        None
    }

    async fn paste_target_app(&self) -> Option<String> {
        None
    }

    async fn set_paste_target_app(&self, _app_name: Option<String>) {}

    async fn focus_app_by_name(&self, _app_name: &str) -> Result<(), WindowError> {
        Err(unavailable())
    }

    async fn frontmost_app_name(&self) -> Result<Option<String>, WindowError> {
        Err(unavailable())
    }

    async fn list_windows(&self) -> Result<Vec<WindowEntry>, WindowError> {
        Err(unavailable())
    }

    async fn focus(&self, _id: &str) -> Result<(), WindowError> {
        Err(unavailable())
    }
}
