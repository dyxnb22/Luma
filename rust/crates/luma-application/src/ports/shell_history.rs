use async_trait::async_trait;
use std::sync::Mutex;
use thiserror::Error;
use tokio_util::sync::CancellationToken;

pub const MAX_SHELL_HISTORY_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_SHELL_HISTORY_ENTRIES: usize = 2_000;
pub const MAX_SHELL_HISTORY_COMMAND_BYTES: usize = 8 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellHistoryEntry {
    pub id: String,
    pub command: String,
    pub timestamp: Option<i64>,
    pub duration_seconds: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellHistorySnapshot {
    pub entries: Vec<ShellHistoryEntry>,
    pub hidden_count: usize,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ShellHistoryError {
    #[error("zsh history is not configured")]
    NotConfigured,
    #[error("zsh history is unavailable: {0}")]
    Unavailable(String),
    #[error("zsh history read cancelled")]
    Cancelled,
}

#[async_trait]
pub trait ShellHistoryPort: Send + Sync {
    async fn read(
        &self,
        cancel: CancellationToken,
    ) -> Result<ShellHistorySnapshot, ShellHistoryError>;
}

pub struct FakeShellHistory {
    snapshot: Mutex<ShellHistorySnapshot>,
    error: Mutex<Option<ShellHistoryError>>,
}

impl FakeShellHistory {
    pub fn new(snapshot: ShellHistorySnapshot) -> Self {
        Self {
            snapshot: Mutex::new(snapshot),
            error: Mutex::new(None),
        }
    }

    pub fn fail_with(&self, error: ShellHistoryError) {
        *self.error.lock().expect("shell history error lock") = Some(error);
    }
}

#[async_trait]
impl ShellHistoryPort for FakeShellHistory {
    async fn read(
        &self,
        cancel: CancellationToken,
    ) -> Result<ShellHistorySnapshot, ShellHistoryError> {
        if cancel.is_cancelled() {
            return Err(ShellHistoryError::Cancelled);
        }
        if let Some(error) = self.error.lock().expect("shell history error lock").take() {
            return Err(error);
        }
        Ok(self
            .snapshot
            .lock()
            .expect("shell history snapshot lock")
            .clone())
    }
}
