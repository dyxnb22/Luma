//! Window catalog port — list + focus visible macOS windows.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WindowEntry {
    /// Stable id, e.g. `pid:123|num:4`.
    pub id: String,
    pub app_name: String,
    pub app_bundle_id: Option<String>,
    pub title: String,
    pub is_on_screen: bool,
    /// CGWindow layer; normal windows are `0`.
    pub layer: i64,
    pub owner_pid: u32,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WindowError {
    #[error("permission required ({capability}): {guidance}")]
    PermissionRequired {
        capability: String,
        guidance: String,
    },
    #[error("unavailable: {0}")]
    Unavailable(String),
    #[error("not found: {0}")]
    NotFound(String),
}

#[async_trait]
pub trait WindowCatalogPort: Send + Sync {
    /// Call once at compose / TUI attach — caches previous-frontmost app label.
    async fn snapshot_previous_frontmost_app(&self) -> Result<Option<String>, WindowError>;

    /// Cached label from [`snapshot_previous_frontmost_app`], if any.
    async fn previous_frontmost_app(&self) -> Option<String>;

    async fn list_windows(&self) -> Result<Vec<WindowEntry>, WindowError>;

    async fn focus(&self, id: &str) -> Result<(), WindowError>;
}

/// Deterministic fake for module / engine tests. Never steals focus.
pub struct FakeWindowCatalog {
    pub entries: tokio::sync::Mutex<Vec<WindowEntry>>,
    pub previous_frontmost: tokio::sync::Mutex<Option<String>>,
    pub list_error: tokio::sync::Mutex<Option<WindowError>>,
    pub focus_error: tokio::sync::Mutex<Option<WindowError>>,
    pub focus_calls: tokio::sync::Mutex<Vec<String>>,
    pub snapshot_calls: tokio::sync::Mutex<u32>,
}

impl Default for FakeWindowCatalog {
    fn default() -> Self {
        Self {
            entries: tokio::sync::Mutex::new(Vec::new()),
            previous_frontmost: tokio::sync::Mutex::new(None),
            list_error: tokio::sync::Mutex::new(None),
            focus_error: tokio::sync::Mutex::new(None),
            focus_calls: tokio::sync::Mutex::new(Vec::new()),
            snapshot_calls: tokio::sync::Mutex::new(0),
        }
    }
}

impl FakeWindowCatalog {
    pub fn with_entries(entries: Vec<WindowEntry>, previous: Option<String>) -> Self {
        Self {
            entries: tokio::sync::Mutex::new(entries),
            previous_frontmost: tokio::sync::Mutex::new(previous),
            ..Default::default()
        }
    }
}

#[async_trait]
impl WindowCatalogPort for FakeWindowCatalog {
    async fn snapshot_previous_frontmost_app(&self) -> Result<Option<String>, WindowError> {
        *self.snapshot_calls.lock().await += 1;
        Ok(self.previous_frontmost.lock().await.clone())
    }

    async fn previous_frontmost_app(&self) -> Option<String> {
        self.previous_frontmost.lock().await.clone()
    }

    async fn list_windows(&self) -> Result<Vec<WindowEntry>, WindowError> {
        if let Some(err) = self.list_error.lock().await.clone() {
            return Err(err);
        }
        Ok(self.entries.lock().await.clone())
    }

    async fn focus(&self, id: &str) -> Result<(), WindowError> {
        self.focus_calls.lock().await.push(id.to_string());
        if let Some(err) = self.focus_error.lock().await.clone() {
            return Err(err);
        }
        Ok(())
    }
}
