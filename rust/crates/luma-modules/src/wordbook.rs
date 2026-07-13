//! Wordbook persistence/review is intentionally unavailable pending a storage migration.
//! Keeping this explicit avoids a hidden SQLite dependency in the modules crate.
use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, LumaModule, ModuleManifest, ModuleState, SearchMode, SearchSink,
    WarmupContext,
};
use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ModuleId, Query, SearchItem};
use luma_protocol::{Event, SearchItemDto};
use tokio_util::sync::CancellationToken;

pub struct WordbookModule {
    manifest: ModuleManifest,
}

impl WordbookModule {
    pub fn new() -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.wordbook"),
                display_name: "Wordbook (unavailable)".into(),
                triggers: vec!["word".into(), "wb".into()],
                default_enabled: false,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
            },
        }
    }
}

impl Default for WordbookModule {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LumaModule for WordbookModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
        ModuleState::Failed("wordbook storage/review migration is not implemented".into())
    }

    async fn search(&self, _query: Query, sink: SearchSink, cancel: CancellationToken) {
        if cancel.is_cancelled() {
            return;
        }
        let _ = sink
            .send(Event::ResultsChunk {
                request_id: String::new(),
                sequence: 1,
                upserts: vec![SearchItemDto {
                    id: "word:unavailable".into(),
                    module_id: "luma.wordbook".into(),
                    title: "Wordbook is unavailable".into(),
                    subtitle: Some("Storage and review migration is pending.".into()),
                    kind: "unavailable".into(),
                    score: 1.0,
                    primary_action_id: "open".into(),
                    primary_action_label: "Details".into(),
                    ..Default::default()
                }],
                removed_ids: vec![],
            })
            .await;
    }

    async fn actions(&self, _result: &SearchItem) -> Vec<ActionDescriptor> {
        vec![ActionDescriptor {
            id: ActionId::new("open"),
            label: "Details".into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        }]
    }

    async fn perform(&self, _action: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if cancel.is_cancelled() {
            ActionOutcome::Cancelled
        } else {
            ActionOutcome::Success {
                message: Some("Wordbook is unavailable pending storage migration.".into()),
            }
        }
    }

    async fn teardown(&self) {}
}
