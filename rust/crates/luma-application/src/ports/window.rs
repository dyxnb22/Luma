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

    /// App name to paste into (updated on snapshot or successful window focus).
    async fn paste_target_app(&self) -> Option<String>;

    async fn set_paste_target_app(&self, app_name: Option<String>);

    /// Focus the first on-screen window owned by `app_name`.
    async fn focus_app_by_name(&self, app_name: &str) -> Result<(), WindowError>;

    /// Current frontmost non-ignored application name, if detectable.
    async fn frontmost_app_name(&self) -> Result<Option<String>, WindowError>;

    async fn list_windows(&self) -> Result<Vec<WindowEntry>, WindowError>;

    async fn focus(&self, id: &str) -> Result<(), WindowError>;

    /// Invalidate pending AX work and wait for an already-started side effect to finish.
    fn abandon_pending_ax_ops(&self) {}
}

/// Deterministic fake for module / engine tests. Never steals focus.
pub struct FakeWindowCatalog {
    pub entries: tokio::sync::Mutex<Vec<WindowEntry>>,
    pub previous_frontmost: tokio::sync::Mutex<Option<String>>,
    pub paste_target: tokio::sync::Mutex<Option<String>>,
    pub list_error: tokio::sync::Mutex<Option<WindowError>>,
    pub focus_error: tokio::sync::Mutex<Option<WindowError>>,
    pub focus_calls: tokio::sync::Mutex<Vec<String>>,
    pub focus_app_calls: tokio::sync::Mutex<Vec<String>>,
    pub snapshot_calls: tokio::sync::Mutex<u32>,
}

impl Default for FakeWindowCatalog {
    fn default() -> Self {
        Self {
            entries: tokio::sync::Mutex::new(Vec::new()),
            previous_frontmost: tokio::sync::Mutex::new(None),
            paste_target: tokio::sync::Mutex::new(None),
            list_error: tokio::sync::Mutex::new(None),
            focus_error: tokio::sync::Mutex::new(None),
            focus_calls: tokio::sync::Mutex::new(Vec::new()),
            focus_app_calls: tokio::sync::Mutex::new(Vec::new()),
            snapshot_calls: tokio::sync::Mutex::new(0),
        }
    }
}

impl FakeWindowCatalog {
    pub fn with_entries(entries: Vec<WindowEntry>, previous: Option<String>) -> Self {
        Self {
            entries: tokio::sync::Mutex::new(entries),
            previous_frontmost: tokio::sync::Mutex::new(previous.clone()),
            paste_target: tokio::sync::Mutex::new(previous),
            ..Default::default()
        }
    }
}

#[async_trait]
impl WindowCatalogPort for FakeWindowCatalog {
    async fn snapshot_previous_frontmost_app(&self) -> Result<Option<String>, WindowError> {
        *self.snapshot_calls.lock().await += 1;
        let label = self.previous_frontmost.lock().await.clone();
        *self.paste_target.lock().await = label.clone();
        Ok(label)
    }

    async fn previous_frontmost_app(&self) -> Option<String> {
        self.previous_frontmost.lock().await.clone()
    }

    async fn paste_target_app(&self) -> Option<String> {
        self.paste_target.lock().await.clone()
    }

    async fn set_paste_target_app(&self, app_name: Option<String>) {
        *self.paste_target.lock().await = app_name;
    }

    async fn focus_app_by_name(&self, app_name: &str) -> Result<(), WindowError> {
        self.focus_app_calls.lock().await.push(app_name.to_string());
        if let Some(err) = self.focus_error.lock().await.clone() {
            return Err(err);
        }
        let entries = self.entries.lock().await;
        let Some(entry) = entries
            .iter()
            .find(|e| e.app_name == app_name && e.is_on_screen)
        else {
            return Err(WindowError::NotFound(format!("app {app_name}")));
        };
        self.focus_calls.lock().await.push(entry.id.clone());
        drop(entries);
        *self.paste_target.lock().await = Some(app_name.to_string());
        Ok(())
    }

    async fn frontmost_app_name(&self) -> Result<Option<String>, WindowError> {
        let entries = self.entries.lock().await;
        Ok(entries
            .iter()
            .find(|e| e.is_on_screen)
            .map(|e| e.app_name.clone()))
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
        let app_name = self
            .entries
            .lock()
            .await
            .iter()
            .find(|e| e.id == id)
            .map(|e| e.app_name.clone());
        if let Some(name) = app_name {
            *self.paste_target.lock().await = Some(name);
        }
        Ok(())
    }
}
