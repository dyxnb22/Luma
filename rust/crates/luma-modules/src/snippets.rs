use crate::cancel::await_unless_cancelled;
use async_trait::async_trait;
use luma_application::{
    AccessibilityPort, ActionOutcome, ActionRequest, LumaModule, ModuleManifest, ModuleState,
    PasteboardPort, SearchMode, SearchSink, SnippetEntry, SnippetsRepository, WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{Event, SearchItemDto};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

type Snippet = SnippetEntry;

pub struct SnippetsModule {
    manifest: ModuleManifest,
    store: Arc<dyn SnippetsRepository>,
    index: RwLock<Vec<Snippet>>,
    store_error: RwLock<Option<String>>,
    pasteboard: Arc<dyn PasteboardPort>,
    accessibility: Arc<dyn AccessibilityPort>,
}

impl SnippetsModule {
    pub fn with_store(
        store: Arc<dyn SnippetsRepository>,
        pasteboard: Arc<dyn PasteboardPort>,
        accessibility: Arc<dyn AccessibilityPort>,
    ) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.snippets"),
                display_name: "Snippets".into(),
                triggers: vec!["s".into(), "snip".into()],
                default_enabled: true,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
                workbench: luma_application::WorkbenchMeta {
                    glyph: Some("S".into()),
                    suggested_query: Some("s ".into()),
                    empty_hint: Some("s · snip add <trigger> <body>".into()),
                    supports_browse: false,
                },
            },
            store,
            index: RwLock::new(Vec::new()),
            store_error: RwLock::new(None),
            pasteboard,
            accessibility,
        }
    }

    async fn refresh_index(&self) -> Result<(), String> {
        match self.store.list() {
            Ok(snippets) => {
                *self.index.write().await = snippets;
                *self.store_error.write().await = None;
                Ok(())
            }
            Err(err) => {
                let msg = err.to_string();
                *self.store_error.write().await = Some(msg.clone());
                Err(msg)
            }
        }
    }

    async fn body_for(&self, trigger: &str) -> Option<String> {
        self.index
            .read()
            .await
            .iter()
            .find(|snippet| snippet.trigger == trigger)
            .map(|snippet| snippet.body.clone())
    }

    async fn upsert(&self, trigger: &str, body: &str) -> Result<(), String> {
        self.store
            .upsert(trigger, body)
            .map_err(|e| e.to_string())?;
        self.refresh_index().await
    }

    async fn delete(&self, trigger: &str) -> Result<(), String> {
        self.store.delete(trigger).map_err(|e| e.to_string())?;
        self.refresh_index().await
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
        // snip add <trigger> <body…>
        let rest_for_add = query.rest_raw();
        if let Some(payload) = rest_for_add
            .strip_prefix("add ")
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            if let Some((trigger, body)) = payload.split_once(char::is_whitespace) {
                let trigger = trigger.trim();
                let body = body.trim();
                if !trigger.is_empty() && !body.is_empty() {
                    let exists = self.index.read().await.iter().any(|s| s.trigger == trigger);
                    let _ = sink
                        .send(Event::ResultsChunk {
                            request_id: String::new(),
                            sequence: 1,
                            upserts: vec![SearchItemDto {
                                id: format!("snip:add:{trigger}"),
                                module_id: "luma.snippets".into(),
                                title: if exists {
                                    format!("Overwrite snippet {trigger}")
                                } else {
                                    format!("Add snippet {trigger}")
                                },
                                subtitle: Some(body.to_string()),
                                kind: if exists {
                                    "update".into()
                                } else {
                                    "create".into()
                                },
                                score: 100.0,
                                primary_action_id: "add".into(),
                                primary_action_label: if exists {
                                    "Overwrite".into()
                                } else {
                                    "Add".into()
                                },
                                primary_action_risk: if exists {
                                    ActionRisk::Confirm
                                } else {
                                    ActionRisk::Safe
                                },
                                primary_action_confirmation: exists,
                                ..Default::default()
                            }],
                            removed_ids: vec![],
                        })
                        .await;
                    return;
                }
            }
        }

        if let Some(err) = self.store_error.read().await.clone() {
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![SearchItemDto {
                        id: "snip:unavailable".into(),
                        module_id: "luma.snippets".into(),
                        title: "Snippets store unavailable".into(),
                        subtitle: Some(err),
                        kind: "unavailable".into(),
                        score: 0.0,
                        primary_action_id: "noop".into(),
                        primary_action_label: "Unavailable".into(),
                        ..Default::default()
                    }],
                    removed_ids: vec![],
                })
                .await;
            return;
        }

        let needle = query.rest_normalized();
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
        if upserts.is_empty() && needle.is_empty() {
            upserts.push(SearchItemDto {
                id: "snip:empty".into(),
                module_id: "luma.snippets".into(),
                title: "No snippets yet".into(),
                subtitle: Some("Add with: snip add <trigger> <body>".into()),
                kind: "onboarding".into(),
                score: 0.0,
                primary_action_id: "noop".into(),
                primary_action_label: "OK".into(),
                ..Default::default()
            });
        } else if upserts.is_empty() {
            upserts.push(SearchItemDto {
                id: "snip:no-matches".into(),
                module_id: "luma.snippets".into(),
                title: format!("No snippets matching \"{needle}\""),
                subtitle: Some("Add with: snip add <trigger> <body>".into()),
                kind: "status".into(),
                score: 0.0,
                primary_action_id: "noop".into(),
                primary_action_label: "OK".into(),
                ..Default::default()
            });
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
    async fn actions(&self, result: &SearchItem) -> Vec<ActionDescriptor> {
        if result.id.as_str() == "snip:empty"
            || result.id.as_str() == "snip:no-matches"
            || result.kind == "onboarding"
            || result.kind == "status"
        {
            return vec![ActionDescriptor {
                id: ActionId::new("noop"),
                label: "OK".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        if result.id.as_str().starts_with("snip:add:") {
            let exists = result.kind == "update";
            return vec![ActionDescriptor {
                id: ActionId::new("add"),
                label: if exists {
                    "Overwrite".into()
                } else {
                    "Add".into()
                },
                risk: if exists {
                    ActionRisk::Confirm
                } else {
                    ActionRisk::Safe
                },
                confirmation: exists,
            }];
        }
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
            ActionDescriptor {
                id: ActionId::new("delete"),
                label: "Delete".into(),
                risk: ActionRisk::Destructive,
                confirmation: true,
            },
        ]
    }
    async fn perform(&self, action: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        match action.action.id.as_str() {
            "noop" => ActionOutcome::Success { message: None },
            "add" => {
                let Some(trigger) = action.result.id.as_str().strip_prefix("snip:add:") else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected snip:add:<trigger>".into(),
                        },
                    };
                };
                let Some(body) = action.result.subtitle.clone() else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "body".into(),
                            message: "missing body".into(),
                        },
                    };
                };
                let exists = self.index.read().await.iter().any(|s| s.trigger == trigger);
                if exists && !action.confirmation {
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "confirmation required to overwrite snippet".into(),
                        },
                    };
                }
                match self.upsert(trigger, &body).await {
                    Ok(()) => ActionOutcome::Success {
                        message: Some(if exists {
                            format!("updated {trigger}")
                        } else {
                            format!("added {trigger}")
                        }),
                    },
                    Err(err) => ActionOutcome::Failed {
                        kind: FailureKind::Io { context: err },
                    },
                }
            }
            "delete" => {
                if action.action.confirmation && !action.confirmation {
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "confirmation required".into(),
                        },
                    };
                }
                let Some(trigger) = action.result.id.as_str().strip_prefix("snip:") else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected snip:<trigger>".into(),
                        },
                    };
                };
                if trigger.starts_with("add:") {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "cannot delete add row".into(),
                        },
                    };
                }
                match self.delete(trigger).await {
                    Ok(()) => ActionOutcome::Success {
                        message: Some(format!("deleted {trigger}")),
                    },
                    Err(err) => ActionOutcome::Failed {
                        kind: FailureKind::Io { context: err },
                    },
                }
            }
            "copy" | "paste" => {
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
                if action.action.id.as_str() == "copy" {
                    return match await_unless_cancelled(&cancel, self.pasteboard.write_text(&body))
                        .await
                    {
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
                    };
                }
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
