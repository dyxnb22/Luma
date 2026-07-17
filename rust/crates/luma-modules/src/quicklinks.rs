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
    store_error: RwLock<Option<String>>,
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
                workbench: luma_application::WorkbenchMeta {
                    glyph: Some("Q".into()),
                    suggested_query: Some("ql ".into()),
                    empty_hint: Some("ql · ql add <trigger> <url>".into()),
                    supports_browse: false,
                },
            },
            store,
            index: RwLock::new(Vec::new()),
            store_error: RwLock::new(None),
            opener,
            pasteboard,
        }
    }

    async fn refresh_index(&self) -> Result<(), String> {
        match self.store.list() {
            Ok(links) => {
                *self.index.write().await = links;
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

fn ql_url_payload(url: &str) -> serde_json::Value {
    serde_json::json!({ "url": url })
}

fn quicklink_subtitle(url: &str) -> String {
    let host = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .or_else(|| url.strip_prefix("mailto:"))
        .unwrap_or(url);
    host.split('/').next().unwrap_or(host).to_string()
}

fn ql_add_payload(trigger: &str, url: &str) -> serde_json::Value {
    serde_json::json!({ "trigger": trigger, "url": url })
}

fn quicklink_trigger_id(id: &str) -> Option<&str> {
    let trigger = id.strip_prefix("ql:")?;
    if trigger.is_empty() || trigger.contains(':') || trigger == "manage" {
        None
    } else {
        Some(trigger)
    }
}

fn url_for_item(item: &SearchItem, index: &[Link]) -> Option<String> {
    if let Some(payload) = &item.action_payload {
        if let Some(url) = payload.get("url").and_then(|v| v.as_str()) {
            return Some(url.to_string());
        }
    }
    if let Some(sub) = &item.subtitle {
        if allowed_scheme(sub) {
            return Some(sub.clone());
        }
    }
    if let Some(trigger) = quicklink_trigger_id(item.id.as_str()) {
        return index
            .iter()
            .find(|l| l.trigger == trigger)
            .map(|l| l.url.clone());
    }
    None
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
                            subtitle: Some(url.clone()),
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
                            ui_intent: None,
                            action_payload: Some(ql_add_payload(&trigger, &url)),
                        }],
                        removed_ids: vec![],
                    })
                    .await;
                return;
            }
        }

        if let Some(err) = self.store_error.read().await.clone() {
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![SearchItemDto {
                        id: "ql:unavailable".into(),
                        module_id: "luma.quicklinks".into(),
                        title: "Quicklinks store unavailable".into(),
                        subtitle: Some(crate::ux::friendly_store_error(&err)),
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
                    subtitle: Some(quicklink_subtitle(&link.url)),
                    kind: "quicklink".into(),
                    score: if link.trigger == token { 90.0 } else { 70.0 },
                    primary_action_id: "open".into(),
                    primary_action_label: "Open".into(),
                    action_payload: Some(ql_url_payload(&link.url)),
                    ..Default::default()
                });
            }
        }
        if token == "ql" || token == "quicklinks" {
            let empty = upserts.is_empty();
            upserts.push(SearchItemDto {
                id: "ql:manage".into(),
                module_id: "luma.quicklinks".into(),
                title: if empty {
                    "No quicklinks yet".into()
                } else {
                    "Add a quicklink".into()
                },
                subtitle: Some("Enter to type: ql add <trigger> <url>".into()),
                kind: if empty {
                    "onboarding".into()
                } else {
                    "status".into()
                },
                score: if empty { 90.0 } else { 5.0 },
                primary_action_id: "seed_add".into(),
                primary_action_label: "Add".into(),
                ..Default::default()
            });
        }
        if !upserts.is_empty() {
            upserts.truncate(query.limit);
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
        if result.id.as_str() == "ql:manage" || result.primary_action.id.as_str() == "seed_add" {
            return vec![ActionDescriptor {
                id: ActionId::new("seed_add"),
                label: "Add".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        if result.id.as_str() == "ql:unavailable"
            || result.kind == "unavailable"
            || result.primary_action.id.as_str() == "noop"
        {
            return vec![ActionDescriptor {
                id: ActionId::new("noop"),
                label: "Unavailable".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        if quicklink_trigger_id(result.id.as_str()).is_some() {
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
                let Some(url) = action
                    .result
                    .action_payload
                    .as_ref()
                    .and_then(|p| p.get("url").and_then(|v| v.as_str()))
                    .map(str::to_string)
                    .or_else(|| action.result.subtitle.clone())
                else {
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
                let Some(trigger) = quicklink_trigger_id(action.result.id.as_str()) else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected ql:<trigger> (not ql:add:*)".into(),
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
            "noop" => ActionOutcome::Success {
                message: Some("ok".into()),
            },
            "seed_add" => ActionOutcome::Failed {
                kind: FailureKind::InvalidInput {
                    field: "action".into(),
                    message: "seed_add is search-driven; use `ql add <trigger> <url>`".into(),
                },
            },
            "open" => {
                let url = {
                    let index = self.index.read().await;
                    url_for_item(&action.result, &index)
                };
                let Some(url) = url else {
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
                let url = {
                    let index = self.index.read().await;
                    url_for_item(&action.result, &index)
                };
                let Some(url) = url else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "url".into(),
                            message: "missing url".into(),
                        },
                    };
                };
                match await_unless_cancelled(&cancel, self.pasteboard.write_text(&url)).await {
                    None => ActionOutcome::Cancelled,
                    Some(Ok(())) => ActionOutcome::Success {
                        message: Some("copied url".into()),
                    },
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
                    entity: format!("action:{other}"),
                },
            },
        }
    }
    async fn teardown(&self) {
        *self.index.write().await = Vec::new();
        *self.store_error.write().await = None;
    }
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

    struct HangPb {
        written: TokioMutex<Option<String>>,
        started: tokio::sync::Notify,
        release: TokioMutex<Option<tokio::sync::oneshot::Receiver<()>>>,
    }

    #[async_trait]
    impl PasteboardPort for HangPb {
        async fn read_text(&self) -> Result<Option<String>, PasteboardError> {
            Ok(self.written.lock().await.clone())
        }
        async fn write_text(&self, text: &str) -> Result<(), PasteboardError> {
            self.started.notify_waiters();
            if let Some(rx) = self.release.lock().await.take() {
                let _ = rx.await;
            }
            *self.written.lock().await = Some(text.into());
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
                ui_intent: None,
                action_payload: None,
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
                        ui_intent: None,
                        action_payload: None,
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
                        ui_intent: None,
                        action_payload: None,
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

    #[tokio::test]
    async fn delete_rejects_ql_add_prefix() {
        let store = Arc::new(MemoryQuicklinksRepository::new());
        let m = QuicklinksModule::with_deps(
            store,
            Arc::new(luma_application::FakeOpenPath::new()),
            Arc::new(MemPb::default()),
        );
        m.upsert("docs", "https://example.com").await.unwrap();
        let item = SearchItem {
            id: luma_domain::ResultId::new("ql:add:docs"),
            module_id: ModuleId::new("luma.quicklinks"),
            title: "Add docs".into(),
            subtitle: None,
            kind: "update".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("delete"),
                label: "Delete".into(),
                risk: ActionRisk::Destructive,
                confirmation: true,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: None,
        };
        let actions = m.actions(&item).await;
        assert!(actions.is_empty() || actions[0].id.as_str() != "delete");
        let outcome = m
            .perform(
                ActionRequest {
                    result: item,
                    action: ActionDescriptor {
                        id: ActionId::new("delete"),
                        label: "Delete".into(),
                        risk: ActionRisk::Destructive,
                        confirmation: true,
                    },
                    confirmation: true,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(
            outcome,
            ActionOutcome::Failed {
                kind: FailureKind::InvalidInput { .. }
            }
        ));
    }

    #[tokio::test]
    async fn quicklinks_copy_cancelled_does_not_write_pasteboard() {
        let store = Arc::new(MemoryQuicklinksRepository::new());
        let (tx, rx) = tokio::sync::oneshot::channel();
        let pb = Arc::new(HangPb {
            written: TokioMutex::new(None),
            started: tokio::sync::Notify::new(),
            release: TokioMutex::new(Some(rx)),
        });
        let m = QuicklinksModule::with_deps(
            store,
            Arc::new(luma_application::FakeOpenPath::new()),
            pb.clone(),
        );
        m.upsert("docs", "https://example.com").await.unwrap();
        let cancel = CancellationToken::new();
        let cancel_c = cancel.clone();
        let started = pb.started.notified();
        tokio::pin!(started);
        let perform = tokio::spawn(async move {
            m.perform(
                ActionRequest {
                    result: SearchItem {
                        id: luma_domain::ResultId::new("ql:docs"),
                        module_id: ModuleId::new("luma.quicklinks"),
                        title: "docs".into(),
                        subtitle: Some("https://example.com".into()),
                        kind: "link".into(),
                        score: 1.0,
                        primary_action: ActionDescriptor {
                            id: ActionId::new("open"),
                            label: "Open".into(),
                            risk: ActionRisk::Safe,
                            confirmation: false,
                        },
                        secondary_actions: vec![],
                        ui_intent: None,
                        action_payload: None,
                    },
                    action: ActionDescriptor {
                        id: ActionId::new("copy"),
                        label: "Copy URL".into(),
                        risk: ActionRisk::Safe,
                        confirmation: false,
                    },
                    confirmation: false,
                },
                cancel,
            )
            .await
        });
        started.await;
        cancel_c.cancel();
        let outcome = perform.await.unwrap();
        assert!(matches!(outcome, ActionOutcome::Cancelled));
        assert!(pb.written.lock().await.is_none());
        let _ = tx.send(());
    }
}

#[cfg(test)]
mod teardown_tests {
    use super::*;
    use luma_application::MemoryQuicklinksRepository;

    #[tokio::test]
    async fn teardown_releases_runtime_caches() {
        let store = Arc::new(MemoryQuicklinksRepository::new());
        store.upsert("docs", "https://example.com").unwrap();
        let m = QuicklinksModule::with_deps(
            store,
            Arc::new(luma_application::FakeOpenPath::new()),
            Arc::new(luma_application::FakePasteboard::new()),
        );
        m.warmup(WarmupContext {
            cancel: CancellationToken::new(),
        })
        .await;
        assert!(!m.index.read().await.is_empty());
        m.teardown().await;
        assert!(m.index.read().await.is_empty());
        assert!(m.store_error.read().await.is_none());
        m.teardown().await;
    }
}
