//! Port for a locally running Mihomo controller.
//!
//! Only redacted, useful proxy data crosses this boundary. Controller authentication material
//! and raw YAML/JSON never cross into the application or module layers.

use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProxyMode {
    Global,
    Rule,
}

impl ProxyMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Rule => "rule",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProxyPorts {
    pub http: Option<u16>,
    pub mixed: Option<u16>,
    pub socks: Option<u16>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProxyStatus {
    pub running: bool,
    pub mode: ProxyMode,
    pub profile: Option<String>,
    pub ports: ProxyPorts,
    pub allow_lan: bool,
    pub tun_enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProxyNode {
    pub name: String,
    pub kind: String,
    pub delay_ms: Option<u32>,
    pub selected: bool,
    pub group: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProxyGroup {
    pub name: String,
    pub selected: Option<String>,
    pub nodes: Vec<ProxyNode>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalControllerStatus {
    pub connected: bool,
    pub endpoint: String,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ProxyCoreError {
    #[error("permission required: {0}")]
    PermissionRequired(String),
    #[error("unavailable: {0}")]
    Unavailable(String),
    #[error("timeout: {0}")]
    Timeout(String),
    #[error("invalid input ({field}): {message}")]
    InvalidInput { field: String, message: String },
    #[error("not found: {0}")]
    NotFound(String),
    #[error("security denied: {0}")]
    SecurityDenied(String),
    #[error("not configured: {0}")]
    NotConfigured(String),
}

#[async_trait]
pub trait ProxyCorePort: Send + Sync {
    async fn get_status(&self) -> Result<ProxyStatus, ProxyCoreError>;
    async fn get_mode(&self) -> Result<ProxyMode, ProxyCoreError>;
    async fn set_mode(&self, mode: ProxyMode) -> Result<(), ProxyCoreError>;
    async fn list_proxies(&self) -> Result<Vec<ProxyNode>, ProxyCoreError>;
    async fn list_proxy_groups(&self) -> Result<Vec<ProxyGroup>, ProxyCoreError>;
    async fn select_proxy(&self, group: &str, proxy: &str) -> Result<(), ProxyCoreError>;
    async fn refresh_provider(&self) -> Result<(), ProxyCoreError>;
    async fn get_external_controller_status(
        &self,
    ) -> Result<ExternalControllerStatus, ProxyCoreError>;
}

/// Deterministic fake used by module and contract tests. It never opens a socket or changes
/// system state.
pub struct FakeProxyCore {
    pub status: Mutex<ProxyStatus>,
    pub groups: Mutex<Vec<ProxyGroup>>,
    pub error: Mutex<Option<ProxyCoreError>>,
    pub selected: Mutex<Vec<(String, String)>>,
    pub mode_changes: Mutex<Vec<ProxyMode>>,
    pub refreshes: Mutex<u32>,
}

impl FakeProxyCore {
    pub fn new(status: ProxyStatus, groups: Vec<ProxyGroup>) -> Arc<Self> {
        Arc::new(Self {
            status: Mutex::new(status),
            groups: Mutex::new(groups),
            error: Mutex::new(None),
            selected: Mutex::new(Vec::new()),
            mode_changes: Mutex::new(Vec::new()),
            refreshes: Mutex::new(0),
        })
    }

    pub async fn set_error(&self, error: Option<ProxyCoreError>) {
        *self.error.lock().await = error;
    }
}

#[async_trait]
impl ProxyCorePort for FakeProxyCore {
    async fn get_status(&self) -> Result<ProxyStatus, ProxyCoreError> {
        if let Some(err) = self.error.lock().await.clone() {
            return Err(err);
        }
        Ok(self.status.lock().await.clone())
    }

    async fn get_mode(&self) -> Result<ProxyMode, ProxyCoreError> {
        Ok(self.get_status().await?.mode)
    }

    async fn set_mode(&self, mode: ProxyMode) -> Result<(), ProxyCoreError> {
        if let Some(err) = self.error.lock().await.clone() {
            return Err(err);
        }
        self.mode_changes.lock().await.push(mode);
        self.status.lock().await.mode = mode;
        Ok(())
    }

    async fn list_proxies(&self) -> Result<Vec<ProxyNode>, ProxyCoreError> {
        if let Some(err) = self.error.lock().await.clone() {
            return Err(err);
        }
        Ok(self
            .groups
            .lock()
            .await
            .iter()
            .flat_map(|group| group.nodes.clone())
            .collect())
    }

    async fn list_proxy_groups(&self) -> Result<Vec<ProxyGroup>, ProxyCoreError> {
        if let Some(err) = self.error.lock().await.clone() {
            return Err(err);
        }
        Ok(self.groups.lock().await.clone())
    }

    async fn select_proxy(&self, group: &str, proxy: &str) -> Result<(), ProxyCoreError> {
        if let Some(err) = self.error.lock().await.clone() {
            return Err(err);
        }
        let mut groups = self.groups.lock().await;
        let Some(target) = groups.iter_mut().find(|g| g.name == group) else {
            return Err(ProxyCoreError::NotFound(format!("proxy group {group}")));
        };
        if !target.nodes.iter().any(|node| node.name == proxy) {
            return Err(ProxyCoreError::NotFound(format!("proxy {proxy}")));
        }
        target.selected = Some(proxy.to_string());
        for node in &mut target.nodes {
            node.selected = node.name == proxy;
        }
        self.selected
            .lock()
            .await
            .push((group.to_string(), proxy.to_string()));
        Ok(())
    }

    async fn refresh_provider(&self) -> Result<(), ProxyCoreError> {
        if let Some(err) = self.error.lock().await.clone() {
            return Err(err);
        }
        *self.refreshes.lock().await += 1;
        Ok(())
    }

    async fn get_external_controller_status(
        &self,
    ) -> Result<ExternalControllerStatus, ProxyCoreError> {
        if let Some(err) = self.error.lock().await.clone() {
            return Err(err);
        }
        Ok(ExternalControllerStatus {
            connected: true,
            endpoint: "fake-controller".into(),
        })
    }
}
