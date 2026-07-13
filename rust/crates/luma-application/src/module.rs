use async_trait::async_trait;
use luma_domain::{ActionDescriptor, FailureKind, ModuleId, Query, SearchItem};
use luma_protocol::Event;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
    TargetedOnly,
    GlobalContributing,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleManifest {
    pub id: ModuleId,
    pub display_name: String,
    pub triggers: Vec<String>,
    pub default_enabled: bool,
    pub search_mode: SearchMode,
    pub required_capabilities: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModuleState {
    Cold,
    Ready,
    Disabled,
    Failed(String),
}

#[derive(Clone, Debug)]
pub struct WarmupContext {
    pub cancel: CancellationToken,
}

#[derive(Clone, Debug)]
pub struct ActionRequest {
    pub result: SearchItem,
    pub action: ActionDescriptor,
    pub confirmation: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionOutcome {
    Success { message: Option<String> },
    Failed { kind: FailureKind },
    Cancelled,
}

pub type SearchSink = mpsc::Sender<Event>;

#[async_trait]
pub trait LumaModule: Send + Sync {
    fn manifest(&self) -> &ModuleManifest;

    async fn warmup(&self, ctx: WarmupContext) -> ModuleState;

    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken);

    async fn actions(&self, result: &SearchItem) -> Vec<ActionDescriptor>;

    async fn perform(&self, action: ActionRequest, cancel: CancellationToken) -> ActionOutcome;

    async fn teardown(&self);
}
