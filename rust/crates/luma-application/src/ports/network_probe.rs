use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NetworkProbeState {
    Pass,
    Fail,
    Skipped,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkProbeStep {
    pub name: String,
    pub state: NetworkProbeState,
    pub detail: String,
    pub remediation: String,
}

#[async_trait]
pub trait NetworkProbePort: Send + Sync {
    async fn base_checks(&self) -> Vec<NetworkProbeStep>;
    async fn loopback_listener(&self, port: u16) -> NetworkProbeStep;
}

pub struct FakeNetworkProbe {
    pub base: Mutex<Vec<NetworkProbeStep>>,
    pub listeners: Mutex<std::collections::HashMap<u16, NetworkProbeStep>>,
}
impl FakeNetworkProbe {
    pub fn new(base: Vec<NetworkProbeStep>) -> Arc<Self> {
        Arc::new(Self {
            base: Mutex::new(base),
            listeners: Mutex::new(Default::default()),
        })
    }
}
#[async_trait]
impl NetworkProbePort for FakeNetworkProbe {
    async fn base_checks(&self) -> Vec<NetworkProbeStep> {
        self.base.lock().await.clone()
    }
    async fn loopback_listener(&self, port: u16) -> NetworkProbeStep {
        self.listeners
            .lock()
            .await
            .get(&port)
            .cloned()
            .unwrap_or(NetworkProbeStep {
                name: format!("Local listener {port}"),
                state: NetworkProbeState::Skipped,
                detail: "not configured".into(),
                remediation: "Configure a local proxy listener first".into(),
            })
    }
}
pub struct UnavailableNetworkProbe;
#[async_trait]
impl NetworkProbePort for UnavailableNetworkProbe {
    async fn base_checks(&self) -> Vec<NetworkProbeStep> {
        vec![NetworkProbeStep {
            name: "Network checks".into(),
            state: NetworkProbeState::Skipped,
            detail: "network probe unavailable".into(),
            remediation: "Use /proxy status after configuring macOS support".into(),
        }]
    }
    async fn loopback_listener(&self, port: u16) -> NetworkProbeStep {
        NetworkProbeStep {
            name: format!("Local listener {port}"),
            state: NetworkProbeState::Skipped,
            detail: "network probe unavailable".into(),
            remediation: "Use /proxy status".into(),
        }
    }
}
