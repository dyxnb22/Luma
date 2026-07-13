use crate::cancel::await_unless_cancelled;
use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, LumaModule, ModuleManifest, ModuleState, OpenPathPort,
    PasteboardPort, QuicklinkEntry, QuicklinksRepository, SearchMode, SearchSink, WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{Event, SearchItemDto};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

type Link = QuicklinkEntry;

pub struct QuicklinksModule {
    manifest: ModuleManifest,
    store: Arc<dyn QuicklinksRepository>,
    index: RwLock<Vec<Link>>,
    opener: Arc<dyn OpenPathPort>,
    pasteboard: Arc<dyn PasteboardPort>,
}

impl QuicklinksModule {
    pub fn with_deps(
        store: Arc<dyn QuicklinksRepository>,
        opener: Arc<dyn OpenPathPort>,
        pasteboard: Arc<dyn PasteboardPort>,
    ) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.quicklinks"),
                display_name: "Quicklinks".into(),
                triggers: vec!["ql".into(), "quicklinks".into()],
                default_enabled: true,
                search_mode: SearchMode::GlobalContributing,
                required_capabilities: vec![],
                workbench: Default::default(),
            },
            store,
            index: RwLock::new(Vec::new()),
            opener,
            pasteboard,
        }
    }

    async fn refresh_index(&self) -> Result<(), String> {
        *self.index.write().await = self.store.list().map_err(|err| err.to_string())?;
        Ok(())
    }

    async fn upsert(&self, trigger: &str, url: &str) -> Result<(), String> {
        self.store
            .upsert(trigger, url)
            .map_err(|err| err.to_string())?;
        self.refresh_index().await
    }

    async fn delete(&self, trigger: &str) -> Result<(), String> {
        self.store.delete(trigger).map_err(|err| err.to_string())?;
        self.refresh_index().await
    }
}

fn allowed_scheme(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://") || url.starts_with("mailto:")
}

#[async_trait]
impl LumaModule for QuicklinksModule {
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
        if cancel.is_cancelled() {
            return;
        }
        let token = query.normalized.split_whitespace().next().unwrap_or("");
        let rest = query.rest_normalized();
        let rest_raw = query.rest_raw();

        if rest.starts_with("add ") {
            let body_raw = rest_raw
                .strip_prefix("add ")
                .or_else(|| rest_raw.strip_prefix("Add "))
                .or_else(|| rest_raw.strip_prefix("ADD "))
                .unwrap_or(rest_raw)
                .trim();
            let parts: Vec<_> = body_raw.split_whitespace().collect();
            if parts.len() >= 2 {
                let trigger = parts[0].to_lowercase();
                let url = parts[1..].join(" ");
                let exists = self
                    .index
                    .read()
                    .await
                    .iter()
                    .any(|link| link.trigger == trigger);
                let (title, action_id, action_label, risk, confirmation) = if exists {
                    (
                        format!("Overwrite {trigger}"),
                        "add",
                        "Overwrite",
                        ActionRisk::Confirm,
                        true,
                    )
                } else {
                    (
                        format!("Add {trigger}"),
                        "add",
                        "Add",
                        ActionRisk::Safe,
                        false,
                    )
                };
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![SearchItemDto {
                            id: format!("ql:add:{trigger}"),
                            module_id: "luma.quicklinks".into(),
                            title,
                            subtitle: Some(url),
                            kind: if exists {
                                "update".into()
                            } else {
                                "create".into()
                            },
                            score: 95.0,
                            primary_action_id: action_id.into(),
                            primary_action_label: action_label.into(),
                            primary_action_risk: risk,
                            primary_action_confirmation: confirmation,
                            secondary_actions: vec![],
                        }],
                        removed_ids: vec![],
                    })
                    .await;
                return;
            }
        }

        let links = self.index.read().await.clone();
        let mut upserts = Vec::new();
        for link in links {
            if cancel.is_cancelled() {
                return;
            }
            if link.trigger == token
                || (!rest.is_empty() && link.trigger.contains(&rest))
                || (token != "ql"
                    && token != "quicklinks"
                    && !token.is_empty()
                    && link.trigger.contains(token))
            {
                upserts.push(SearchItemDto {
                    id: format!("ql:{}", link.trigger),
                    module_id: "luma.quicklinks".into(),
                    title: link.trigger.clone(),
                    subtitle: Some(link.url.clone()),
                    kind: "quicklink".into(),
                    score: if link.trigger == token { 90.0 } else { 70.0 },
                    primary_action_id: "open".into(),
                    primary_action_label: "Open".into(),
                    ..Default::default()
                });
            }
        }
        if token == "ql" || token == "quicklinks" {
            upserts.push(SearchItemDto {
                id: "ql:manage".into(),
                module_id: "luma.quicklinks".into(),
                title: "Manage Quicklinks".into(),
                subtitle: Some("ql add <trigger> <url>".into()),
                kind: "open".into(),
                score: 1.0,
                primary_action_id: "open".into(),
                primary_action_label: "Open".into(),
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
        if let Some(trigger) = result.id.as_str().strip_prefix("ql:add:") {
            let exists = self
                .index
                .read()
                .await
                .iter()
                .any(|link| link.trigger == trigger);
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
        let mut actions = vec![
            ActionDescriptor {
                id: ActionId::new("open"),
                label: "Open".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            ActionDescriptor {
                id: ActionId::new("copy"),
                label: "Copy URL".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
        ];
        if result.id.as_str().starts_with("ql:") && result.id.as_str() != "ql:manage" {
            actions.push(ActionDescriptor {
                id: ActionId::new("delete"),
                label: "Delete".into(),
                risk: ActionRisk::Destructive,
                confirmation: true,
            });
        }
        actions
    }
    async fn perform(&self, action: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        match action.action.id.as_str() {
            "add" => {
                let Some(trigger) = action.result.id.as_str().strip_prefix("ql:add:") else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected ql:add:<trigger>".into(),
                        },
                    };
                };
                let Some(url) = action.result.subtitle.clone() else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "url".into(),
                            message: "missing url".into(),
                        },
                    };
                };
                if !allowed_scheme(&url) {
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "URL scheme not allowed".into(),
                        },
                    };
                }
                let exists = self
                    .index
                    .read()
                    .await
                    .iter()
                    .any(|link| link.trigger == trigger);
                if exists && !action.confirmation {
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "confirmation required to overwrite existing quicklink".into(),
                        },
                    };
                }
                if cancel.is_cancelled() {
                    return ActionOutcome::Cancelled;
                }
                match self.upsert(trigger, &url).await {
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
                let Some(trigger) = action.result.id.as_str().strip_prefix("ql:") else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected ql:<trigger>".into(),
                        },
                    };
                };
                match self.delete(trigger).await {
                    Ok(()) => ActionOutcome::Success {
                        message: Some(format!("deleted {trigger}")),
                    },
                    Err(err) => ActionOutcome::Failed {
                        kind: FailureKind::Io { context: err },
                    },
                }
            }
            "open" => {
                if action.result.id.as_str() == "ql:manage" {
                    return ActionOutcome::Success {
                        message: Some("use ql add <trigger> <url>".into()),
                    };
                }
                let Some(url) = action.result.subtitle.clone() else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "url".into(),
                            message: "missing url".into(),
                        },
                    };
                };
                if !allowed_scheme(&url) {
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "URL scheme not allowed".into(),
                        },
                    };
                }
                match await_unless_cancelled(&cancel, self.opener.open(Path::new(&url))).await {
                    None => ActionOutcome::Cancelled,
                    Some(Ok(())) => ActionOutcome::Success {
                        message: Some(format!("opened {url}")),
                    },
                    Some(Err(err)) => ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: err.to_string(),
                            retryable: true,
                        },
                    },
                }
            }
            "copy" => {
                let Some(url) = action.result.subtitle.clone() else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "url".into(),
                            message: "missing url".into(),
                        },
                    };
                };
                match self.pasteboard.write_text(&url).await {
                    Ok(()) => ActionOutcome::Success {
                        message: Some("copied url".into()),
                    },
                    Err(err) => ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: err.to_string(),
                            retryable: true,
                        },
                    },
                }
            }
            other => ActionOutcome::Failed {
                kind: FailureKind::NotFound {
                    entity: format!("action:{other}"),
                },
            },
        }
    }
    async fn teardown(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use luma_application::{MemoryQuicklinksRepository, PasteboardError};
    use tokio::sync::Mutex as TokioMutex;

    #[derive(Default)]
    struct MemPb(TokioMutex<Option<String>>);

    #[async_trait]
    impl PasteboardPort for MemPb {
        async fn read_text(&self) -> Result<Option<String>, PasteboardError> {
            Ok(self.0.lock().await.clone())
        }
        async fn write_text(&self, text: &str) -> Result<(), PasteboardError> {
            *self.0.lock().await = Some(text.into());
            Ok(())
        }
    }

    #[tokio::test]
    async fn upsert_and_search_index() {
        let store = Arc::new(MemoryQuicklinksRepository::new());
        let m = QuicklinksModule::with_deps(
            store.clone(),
            Arc::new(luma_application::FakeOpenPath::new()),
            Arc::new(MemPb::default()),
        );
        m.upsert("docs", "https://example.com").await.unwrap();
        // Index is independent of the backing store after upsert.
        store.delete("docs").unwrap();
        assert!(m.index.read().await.iter().any(|l| l.trigger == "docs"));
    }

    #[tokio::test]
    async fn overwrite_requires_confirmation() {
        let store = Arc::new(MemoryQuicklinksRepository::new());
        let m = QuicklinksModule::with_deps(
            store,
            Arc::new(luma_application::FakeOpenPath::new()),
            Arc::new(MemPb::default()),
        );
        m.upsert("docs", "https://example.com").await.unwrap();
        let actions = m
            .actions(&SearchItem {
                id: luma_domain::ResultId::new("ql:add:docs"),
                module_id: ModuleId::new("luma.quicklinks"),
                title: "Overwrite docs".into(),
                subtitle: Some("https://other.example".into()),
                kind: "update".into(),
                score: 1.0,
                primary_action: ActionDescriptor {
                    id: ActionId::new("add"),
                    label: "Overwrite".into(),
                    risk: ActionRisk::Confirm,
                    confirmation: true,
                },
                secondary_actions: vec![],
            })
            .await;
        assert_eq!(actions.len(), 1);
        assert!(actions[0].confirmation);
        assert_eq!(actions[0].risk, ActionRisk::Confirm);

        let denied = m
            .perform(
                ActionRequest {
                    result: SearchItem {
                        id: luma_domain::ResultId::new("ql:add:docs"),
                        module_id: ModuleId::new("luma.quicklinks"),
                        title: "Overwrite docs".into(),
                        subtitle: Some("https://other.example".into()),
                        kind: "update".into(),
                        score: 1.0,
                        primary_action: actions[0].clone(),
                        secondary_actions: vec![],
                    },
                    action: actions[0].clone(),
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(
            denied,
            ActionOutcome::Failed {
                kind: FailureKind::SecurityDenied { .. }
            }
        ));

        let ok = m
            .perform(
                ActionRequest {
                    result: SearchItem {
                        id: luma_domain::ResultId::new("ql:add:docs"),
                        module_id: ModuleId::new("luma.quicklinks"),
                        title: "Overwrite docs".into(),
                        subtitle: Some("https://other.example".into()),
                        kind: "update".into(),
                        score: 1.0,
                        primary_action: actions[0].clone(),
                        secondary_actions: vec![],
                    },
                    action: actions[0].clone(),
                    confirmation: true,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(ok, ActionOutcome::Success { .. }));
        assert_eq!(
            m.index
                .read()
                .await
                .iter()
                .find(|l| l.trigger == "docs")
                .unwrap()
                .url,
            "https://other.example"
        );
    }
}
