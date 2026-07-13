use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, LumaModule, ModuleManifest, ModuleState, SearchMode, SearchSink,
    WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_platform_macos::{MacTranslator, Translator};
use luma_protocol::{Event, SearchItemDto};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub struct TranslateModule {
    manifest: ModuleManifest,
    translator: Arc<dyn Translator>,
    target_language: String,
}

impl TranslateModule {
    pub fn new() -> Self {
        Self::with_translator(Arc::new(MacTranslator), "en".into())
    }

    pub fn with_translator(translator: Arc<dyn Translator>, target_language: String) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.translate"),
                display_name: "Translate".into(),
                triggers: vec!["tr".into(), "translate".into()],
                default_enabled: true,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec!["translation".into()],
            },
            translator,
            target_language,
        }
    }
}

impl Default for TranslateModule {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LumaModule for TranslateModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }
    async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
        ModuleState::Ready
    }
    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        if cancel.is_cancelled() {
            return;
        }
        let text = query
            .normalized
            .split_once(|c: char| c.is_whitespace())
            .map(|(_, r)| r.trim().to_string())
            .unwrap_or_default();
        let title = if text.is_empty() {
            "Translate".into()
        } else {
            format!("Translate: {text}")
        };
        let _ = sink
            .send(Event::ResultsChunk {
                request_id: String::new(),
                sequence: 1,
                upserts: vec![SearchItemDto {
                    id: format!("tr:{text}"),
                    module_id: "luma.translate".into(),
                    title,
                    subtitle: Some(format!("target={}", self.target_language)),
                    kind: "translate".into(),
                    score: 40.0,
                    primary_action_id: "translate".into(),
                    primary_action_label: "Translate".into(),
                }],
                removed_ids: vec![],
            })
            .await;
    }
    async fn actions(&self, _result: &SearchItem) -> Vec<ActionDescriptor> {
        vec![ActionDescriptor {
            id: ActionId::new("translate"),
            label: "Translate".into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        }]
    }
    async fn perform(&self, action: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        let text = action
            .result
            .id
            .as_str()
            .strip_prefix("tr:")
            .unwrap_or("")
            .to_string();
        match self
            .translator
            .translate(&text, &self.target_language)
            .await
        {
            Ok(result) => ActionOutcome::Success {
                message: Some(result.translated_text),
            },
            Err(luma_platform_macos::TranslationError::EmptyInput) => ActionOutcome::Failed {
                kind: FailureKind::InvalidInput {
                    field: "text".into(),
                    message: "empty translation input".into(),
                },
            },
            Err(luma_platform_macos::TranslationError::Unavailable(reason)) => {
                ActionOutcome::Failed {
                    kind: FailureKind::Unavailable {
                        reason,
                        retryable: true,
                    },
                }
            }
        }
    }
    async fn teardown(&self) {}
}
