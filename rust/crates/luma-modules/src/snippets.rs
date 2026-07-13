use crate::cancel::await_unless_cancelled;
use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, LumaModule, ModuleManifest, ModuleState, SearchMode, SearchSink,
    WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_platform_macos::{Accessibility, Pasteboard};
use luma_protocol::{Event, SearchItemDto};
use luma_storage::{SnippetRow, SnippetsStore};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

type Snippet = SnippetRow;

pub struct SnippetsModule {
    manifest: ModuleManifest,
    store: Arc<SnippetsStore>,
    index: RwLock<Vec<Snippet>>,
    pasteboard: Arc<dyn Pasteboard>,
    accessibility: Arc<dyn Accessibility>,
}

impl SnippetsModule {
    pub fn with_store(
        store: Arc<SnippetsStore>,
        pasteboard: Arc<dyn Pasteboard>,
        accessibility: Arc<dyn Accessibility>,
    ) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.snippets"),
                display_name: "Snippets".into(),
                triggers: vec!["s".into(), "snip".into()],
                default_enabled: true,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
            },
            store,
            index: RwLock::new(Vec::new()),
            pasteboard,
            accessibility,
        }
    }

    async fn refresh_index(&self) -> Result<(), String> {
        *self.index.write().await = self.store.list().map_err(|err| err.to_string())?;
        Ok(())
    }

    async fn body_for(&self, trigger: &str) -> Option<String> {
        self.index
            .read()
            .await
            .iter()
            .find(|snippet| snippet.trigger == trigger)
            .map(|snippet| snippet.body.clone())
    }
}

#[async_trait]
impl LumaModule for SnippetsModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }
    async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
        match self.refresh_index().await {
            Ok(()) => ModuleState::Ready,
            Err(err) => ModuleState::Failed(err),
        }
    }
    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        let needle = query
            .normalized
            .split_once(|c: char| c.is_whitespace())
            .map(|(_, r)| r.trim().to_string())
            .unwrap_or_default();
        let snippets = self.index.read().await.clone();
        let mut upserts = Vec::new();
        for snip in snippets {
            if cancel.is_cancelled() {
                return;
            }
            if needle.is_empty()
                || snip.trigger.contains(&needle)
                || snip.body.to_lowercase().contains(&needle)
            {
                upserts.push(SearchItemDto {
                    id: format!("snip:{}", snip.trigger),
                    module_id: "luma.snippets".into(),
                    title: snip.trigger,
                    subtitle: Some(snip.body),
                    kind: "snippet".into(),
                    score: 55.0,
                    primary_action_id: "copy".into(),
                    primary_action_label: "Copy".into(),
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
        vec![
            ActionDescriptor {
                id: ActionId::new("copy"),
                label: "Copy".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            ActionDescriptor {
                id: ActionId::new("paste"),
                label: "Paste".into(),
                risk: ActionRisk::Confirm,
                confirmation: false,
            },
        ]
    }
    async fn perform(&self, action: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        let Some(trigger) = action.result.id.as_str().strip_prefix("snip:") else {
            return ActionOutcome::Failed {
                kind: FailureKind::NotFound {
                    entity: action.result.id.as_str().into(),
                },
            };
        };
        let Some(body) = self.body_for(trigger).await else {
            return ActionOutcome::Failed {
                kind: FailureKind::NotFound {
                    entity: action.result.id.as_str().into(),
                },
            };
        };
        match action.action.id.as_str() {
            "copy" => {
                match await_unless_cancelled(&cancel, self.pasteboard.write_text(&body)).await {
                    None => ActionOutcome::Cancelled,
                    Some(Ok(())) => ActionOutcome::Success {
                        message: Some("copied".into()),
                    },
                    Some(Err(err)) => ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: err.to_string(),
                            retryable: true,
                        },
                    },
                }
            }
            "paste" => {
                if !self.accessibility.is_trusted() {
                    return ActionOutcome::Failed {
                        kind: FailureKind::PermissionRequired {
                            capability: "accessibility".into(),
                            guidance: "Grant Accessibility to paste".into(),
                        },
                    };
                }
                match await_unless_cancelled(&cancel, self.pasteboard.write_text(&body)).await {
                    None => ActionOutcome::Cancelled,
                    Some(Ok(())) => {
                        if cancel.is_cancelled() {
                            return ActionOutcome::Cancelled;
                        }
                        match await_unless_cancelled(&cancel, self.accessibility.paste_clipboard())
                            .await
                        {
                            None => ActionOutcome::Cancelled,
                            Some(Ok(())) => ActionOutcome::Success {
                                message: Some("pasted".into()),
                            },
                            Some(Err(_)) => ActionOutcome::Failed {
                                kind: FailureKind::PermissionRequired {
                                    capability: "accessibility".into(),
                                    guidance: "Grant Accessibility to paste".into(),
                                },
                            },
                        }
                    }
                    Some(Err(err)) => ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: err.to_string(),
                            retryable: true,
                        },
                    },
                }
            }
            other => ActionOutcome::Failed {
                kind: FailureKind::NotFound {
                    entity: other.into(),
                },
            },
        }
    }
    async fn teardown(&self) {}
}
