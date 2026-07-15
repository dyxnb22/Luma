//! Port for the user's macOS HTTP/SOCKS system proxy settings.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemProxySetting {
    pub enabled: bool,
    pub server: Option<String>,
    pub port: Option<u16>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemProxyStatus {
    pub service: String,
    pub http: SystemProxySetting,
    pub socks: SystemProxySetting,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum SystemProxyError {
    #[error("permission required: {0}")]
    PermissionRequired(String),
    #[error("unavailable: {0}")]
    Unavailable(String),
    #[error("invalid input ({field}): {message}")]
    InvalidInput { field: String, message: String },
    #[error("proxy settings changed outside Luma; left untouched")]
    Conflict,
}

#[async_trait]
pub trait SystemProxyPort: Send + Sync {
    async fn get_status(&self) -> Result<SystemProxyStatus, SystemProxyError>;
    async fn enable(
        &self,
        http_port: Option<u16>,
        socks_port: Option<u16>,
    ) -> Result<SystemProxyStatus, SystemProxyError>;
    async fn disable(&self) -> Result<SystemProxyStatus, SystemProxyError>;
}

/// In-memory system proxy adapter for tests. It records mutations but never calls macOS.
pub struct FakeSystemProxy {
    pub status: Mutex<SystemProxyStatus>,
    pub error: Mutex<Option<SystemProxyError>>,
    pub enable_calls: Mutex<Vec<(Option<u16>, Option<u16>)>>,
    pub disable_calls: Mutex<u32>,
    pub applied: Mutex<Option<SystemProxyStatus>>,
}

impl FakeSystemProxy {
    pub fn new(status: SystemProxyStatus) -> Arc<Self> {
        Arc::new(Self {
            status: Mutex::new(status),
            error: Mutex::new(None),
            enable_calls: Mutex::new(Vec::new()),
            disable_calls: Mutex::new(0),
            applied: Mutex::new(None),
        })
    }

    pub async fn set_error(&self, error: Option<SystemProxyError>) {
        *self.error.lock().await = error;
    }
}

#[async_trait]
impl SystemProxyPort for FakeSystemProxy {
    async fn get_status(&self) -> Result<SystemProxyStatus, SystemProxyError> {
        if let Some(err) = self.error.lock().await.clone() {
            return Err(err);
        }
        Ok(self.status.lock().await.clone())
    }

    async fn enable(
        &self,
        http_port: Option<u16>,
        socks_port: Option<u16>,
    ) -> Result<SystemProxyStatus, SystemProxyError> {
        if let Some(err) = self.error.lock().await.clone() {
            return Err(err);
        }
        self.enable_calls.lock().await.push((http_port, socks_port));
        let mut status = self.status.lock().await;
        if let Some(port) = http_port {
            status.http = SystemProxySetting {
                enabled: true,
                server: Some("127.0.0.1".into()),
                port: Some(port),
            };
        }
        if let Some(port) = socks_port {
            status.socks = SystemProxySetting {
                enabled: true,
                server: Some("127.0.0.1".into()),
                port: Some(port),
            };
        }
        *self.applied.lock().await = Some(status.clone());
        Ok(status.clone())
    }

    async fn disable(&self) -> Result<SystemProxyStatus, SystemProxyError> {
        if let Some(err) = self.error.lock().await.clone() {
            return Err(err);
        }
        if self.applied.lock().await.is_none() {
            return Err(SystemProxyError::Conflict);
        }
        *self.disable_calls.lock().await += 1;
        let mut status = self.status.lock().await;
        status.http.enabled = false;
        status.socks.enabled = false;
        *self.applied.lock().await = None;
        Ok(status.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn disable_without_luma_ownership_is_a_conflict() {
        let fake = FakeSystemProxy::new(SystemProxyStatus {
            service: "Wi-Fi".into(),
            http: SystemProxySetting {
                enabled: true,
                server: Some("127.0.0.1".into()),
                port: Some(7899),
            },
            socks: SystemProxySetting::default(),
        });
        assert_eq!(fake.disable().await, Err(SystemProxyError::Conflict));
    }
}
