use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, LumaModule, ModuleManifest, ModuleState, SearchMode, SearchSink,
    WarmupContext,
};
use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ModuleId, Query, SearchItem};
use luma_protocol::{Event, SearchItemDto};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
struct Record {
    id: String,
    title: String,
}

pub struct MediaModule {
    manifest: ModuleManifest,
    items: Arc<RwLock<Vec<Record>>>,
}

impl MediaModule {
    pub fn new() -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.media"),
                display_name: "Records".into(),
                triggers: vec!["rec".into(), "m".into(), "media".into()],
                default_enabled: false,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
            },
            items: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl Default for MediaModule {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LumaModule for MediaModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }
    async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
        ModuleState::Ready
    }
    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        let needle = query
            .normalized
            .split_once(|c: char| c.is_whitespace())
            .map(|(_, r)| r.trim().to_string())
            .unwrap_or_default();
        let items = self.items.read().await.clone();
        if items.is_empty() {
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![SearchItemDto {
                        id: "media:manage".into(),
                        module_id: "luma.media".into(),
                        title: "Records (empty)".into(),
                        subtitle: Some("add via rec <title>".into()),
                        kind: "open".into(),
                        score: 1.0,
                        primary_action_id: "open".into(),
                        primary_action_label: "Open".into(),
                        ..Default::default()
                    }],
                    removed_ids: vec![],
                })
                .await;
            return;
        }
        let mut upserts = Vec::new();
        for item in items {
            if cancel.is_cancelled() {
                return;
            }
            if needle.is_empty() || item.title.to_lowercase().contains(&needle) {
                upserts.push(SearchItemDto {
                    id: format!("media:{}", item.id),
                    module_id: "luma.media".into(),
                    title: item.title,
                    subtitle: None,
                    kind: "media".into(),
                    score: 45.0,
                    primary_action_id: "open".into(),
                    primary_action_label: "Open".into(),
                    ..Default::default()
                });
            }
        }
        if !upserts.is_empty() {
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts,
                    removed_ids: vec![],
                })
                .await;
        }
    }
    async fn actions(&self, _result: &SearchItem) -> Vec<ActionDescriptor> {
        vec![ActionDescriptor {
            id: ActionId::new("open"),
            label: "Open".into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        }]
    }
    async fn perform(&self, _action: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        ActionOutcome::Success {
            message: Some("records route".into()),
        }
    }
    async fn teardown(&self) {}
}
