use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeListener {
    pub port: u16,
    pub address: String,
    pub pid: u32,
    pub process_name: String,
    pub user: Option<String>,
    pub cwd: Option<PathBuf>,
    /// Captured at listing time and rechecked immediately before SIGTERM.
    pub identity: String,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum RuntimeError {
    #[error("permission required: {0}")]
    PermissionRequired(String),
    #[error("runtime unavailable: {0}")]
    Unavailable(String),
    #[error("listener no longer exists")]
    NotFound,
    #[error("refusing to terminate: {0}")]
    SecurityDenied(String),
    #[error("operation timed out")]
    Timeout,
}

#[async_trait]
pub trait RuntimePort: Send + Sync {
    async fn list_tcp_listeners(&self) -> Result<Vec<RuntimeListener>, RuntimeError>;
    /// Implementations re-list and compare identity before sending only SIGTERM.
    async fn terminate_gracefully(&self, listener: RuntimeListener) -> Result<(), RuntimeError>;
}

pub struct FakeRuntimePort {
    pub listeners: Mutex<Vec<RuntimeListener>>,
    pub terminated: Mutex<Vec<u32>>,
    pub error: Mutex<Option<RuntimeError>>,
}

impl FakeRuntimePort {
    pub fn new(listeners: Vec<RuntimeListener>) -> Arc<Self> {
        Arc::new(Self {
            listeners: Mutex::new(listeners),
            terminated: Mutex::new(vec![]),
            error: Mutex::new(None),
        })
    }
}

#[async_trait]
impl RuntimePort for FakeRuntimePort {
    async fn list_tcp_listeners(&self) -> Result<Vec<RuntimeListener>, RuntimeError> {
        if let Some(error) = self.error.lock().await.clone() {
            return Err(error);
        }
        Ok(self.listeners.lock().await.clone())
    }
    async fn terminate_gracefully(&self, listener: RuntimeListener) -> Result<(), RuntimeError> {
        if let Some(error) = self.error.lock().await.clone() {
            return Err(error);
        }
        if !self
            .listeners
            .lock()
            .await
            .iter()
            .any(|item| item.pid == listener.pid && item.identity == listener.identity)
        {
            return Err(RuntimeError::NotFound);
        }
        self.terminated.lock().await.push(listener.pid);
        Ok(())
    }
}
