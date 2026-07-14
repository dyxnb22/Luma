use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessEntry {
    pub pid: u32,
    /// Short display name (basename).
    pub name: String,
    /// Full executable path / command from the process table.
    pub executable: String,
    /// Approximate process start unix seconds (now − etimes at list time).
    pub start_unix: i64,
}

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("unavailable: {0}")]
    Unavailable(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[async_trait]
pub trait ProcessCatalogPort: Send + Sync {
    async fn list_gui_ish(&self) -> Result<Vec<ProcessEntry>, ProcessError>;
    async fn quit(&self, pid: u32, force: bool) -> Result<(), ProcessError>;
}

pub struct FakeProcessCatalog {
    pub processes: tokio::sync::Mutex<Vec<ProcessEntry>>,
    pub list_error: Option<String>,
    pub quit_error: Option<String>,
    pub quit_calls: tokio::sync::Mutex<Vec<(u32, bool)>>,
}

#[async_trait]
impl ProcessCatalogPort for FakeProcessCatalog {
    async fn list_gui_ish(&self) -> Result<Vec<ProcessEntry>, ProcessError> {
        if let Some(msg) = &self.list_error {
            return Err(ProcessError::Unavailable(msg.clone()));
        }
        Ok(self.processes.lock().await.clone())
    }

    async fn quit(&self, pid: u32, force: bool) -> Result<(), ProcessError> {
        self.quit_calls.lock().await.push((pid, force));
        if let Some(msg) = &self.quit_error {
            return Err(ProcessError::Unavailable(msg.clone()));
        }
        Ok(())
    }
}
