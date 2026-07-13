use async_trait::async_trait;
use luma_application::{
    looks_secret, AccessibilityPort, ActionOutcome, ActionRequest, ClipboardEntry,
    ClipboardHistoryRepository, ClipboardRepoError, LumaModule, ModuleManifest, ModuleState,
    PasteboardPort, SearchMode, SearchSink, WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{Event, SearchItemDto};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::clipboard_privacy::ClipboardSuppression;

const DEFAULT_RETENTION_DAYS: u32 = 30;

pub struct ClipboardModule {
    manifest: ModuleManifest,
    store: Arc<dyn ClipboardHistoryRepository>,
    pasteboard: Arc<dyn PasteboardPort>,
    accessibility: Arc<dyn AccessibilityPort>,
    suppression: Arc<ClipboardSuppression>,
    retention_days: u32,
    last_seen_text: Arc<Mutex<Option<String>>>,
    index: Arc<RwLock<Vec<ClipboardEntry>>>,
    poll_cancel: Mutex<Option<CancellationToken>>,
    poll_handle: Mutex<Option<JoinHandle<()>>>,
}

impl ClipboardModule {
    pub fn with_deps(
        store: Arc<dyn ClipboardHistoryRepository>,
        pasteboard: Arc<dyn PasteboardPort>,
        accessibility: Arc<dyn AccessibilityPort>,
        suppression: Arc<ClipboardSuppression>,
    ) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.clipboard"),
                display_name: "Clipboard".into(),
                triggers: vec!["clip".into(), "cb".into()],
                default_enabled: true,
                search_mode: SearchMode::GlobalContributing,
                required_capabilities: vec![],
            },
            store,
            pasteboard,
            accessibility,
            suppression,
            retention_days: DEFAULT_RETENTION_DAYS,
            last_seen_text: Arc::new(Mutex::new(None)),
            index: Arc::new(RwLock::new(Vec::new())),
            poll_cancel: Mutex::new(None),
            poll_handle: Mutex::new(None),
        }
    }

    pub fn suppression(&self) -> Arc<ClipboardSuppression> {
        self.suppression.clone()
    }

    async fn refresh_index(
        store: &dyn ClipboardHistoryRepository,
        index: &RwLock<Vec<ClipboardEntry>>,
    ) -> Result<(), ClipboardRepoError> {
        *index.write().await = store.list_page(0, 500)?;
        Ok(())
    }

    async fn capture_once(
        store: &dyn ClipboardHistoryRepository,
        index: &RwLock<Vec<ClipboardEntry>>,
        pasteboard: &dyn PasteboardPort,
        suppression: &ClipboardSuppression,
        last_seen_text: &Mutex<Option<String>>,
        retention_days: u32,
    ) {
        if let Ok(Some(text)) = pasteboard.read_text().await {
            if looks_secret(&text) || suppression.is_suppressed(&text) {
                let mut last = last_seen_text.lock().await;
                *last = Some(text);
                return;
            }
            {
                let mut last = last_seen_text.lock().await;
                if last.as_ref() == Some(&text) {
                    return;
                }
                *last = Some(text.clone());
            }
            if let Ok(Some(latest)) = store.latest_by_created() {
                if latest.text == text {
                    return;
                }
            }
            let _ = store.purge_older_than_days(retention_days);
            if store.insert(&text, false).is_ok() {
                let _ = Self::refresh_index(store, index).await;
            }
        }
    }

    async fn start_poller(&self, parent: CancellationToken) {
        self.stop_poller().await;
        let cancel = parent.child_token();
        let store = self.store.clone();
        let index = self.index.clone();
        let pasteboard = self.pasteboard.clone();
        let suppression = self.suppression.clone();
        let last_seen_text = self.last_seen_text.clone();
        let retention_days = self.retention_days;
        let token = cancel.clone();
        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = token.cancelled() => break,
                    _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                        Self::capture_once(
                            store.as_ref(),
                            &index,
                            pasteboard.as_ref(),
                            &suppression,
                            &last_seen_text,
                            retention_days,
                        ).await;
                    }
                }
            }
        });
        *self.poll_cancel.lock().await = Some(cancel);
        *self.poll_handle.lock().await = Some(handle);
    }

    async fn stop_poller(&self) {
        if let Some(cancel) = self.poll_cancel.lock().await.take() {
            cancel.cancel();
        }
        if let Some(handle) = self.poll_handle.lock().await.take() {
            let _ = handle.await;
        }
    }
}

#[async_trait]
impl LumaModule for ClipboardModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, ctx: WarmupContext) -> ModuleState {
        if !ctx.cancel.is_cancelled() {
            if let Err(err) = Self::refresh_index(self.store.as_ref(), &self.index).await {
                return ModuleState::Failed(err.to_string());
            }
            // Seed last_seen from the current pasteboard without capturing into history.
            // Avoids writing a pre-existing secret (e.g. after Secrets copy + restart).
            if let Ok(Some(text)) = self.pasteboard.read_text().await {
                *self.last_seen_text.lock().await = Some(text);
            }
            let _ = self.store.purge_older_than_days(self.retention_days);
            self.start_poller(ctx.cancel).await;
        }
        ModuleState::Ready
    }

    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        let needle = match &query.scope {
            luma_domain::QueryScope::Targeted { .. } => query
                .normalized
                .split_once(|c: char| c.is_whitespace())
                .map(|(_, rest)| rest.trim().to_string())
                .unwrap_or_default(),
            luma_domain::QueryScope::Global => {
                if query.normalized.chars().count() < 3 {
                    return;
                }
                query.normalized.clone()
            }
        };

        let limit = if matches!(query.scope, luma_domain::QueryScope::Global) {
            query.limit.min(3)
        } else {
            query.limit
        };
        let needle = needle.to_lowercase();
        if needle == "clear" {
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![SearchItemDto {
                        id: "clip:clear".into(),
                        module_id: "luma.clipboard".into(),
                        title: "Clear unpinned clipboard history".into(),
                        subtitle: Some("keeps pinned items".into()),
                        kind: "manage".into(),
                        score: 100.0,
                        primary_action_id: "clear".into(),
                        primary_action_label: "Clear unpinned".into(),
                        primary_action_risk: ActionRisk::Destructive,
                        primary_action_confirmation: true,
                        ..Default::default()
                    }],
                    removed_ids: vec![],
                })
                .await;
            return;
        }
        let mut rows: Vec<_> = self
            .index
            .read()
            .await
            .iter()
            .filter(|entry| needle.is_empty() || entry.text.to_lowercase().contains(&needle))
            .cloned()
            .collect();
        rows.sort_by_key(|entry| (!entry.pinned, std::cmp::Reverse(entry.created_at)));
        rows.truncate(limit);
        let mut upserts = Vec::new();
        for entry in rows {
            if cancel.is_cancelled() {
                return;
            }
            let preview: String = entry.text.chars().take(80).collect();
            let subtitle = if entry.pinned {
                Some("pinned".into())
            } else {
                None
            };
            upserts.push(SearchItemDto {
                id: format!("clip:{}", entry.id),
                module_id: "luma.clipboard".into(),
                title: preview,
                subtitle,
                kind: "clip".into(),
                score: if needle.is_empty() { 50.0 } else { 60.0 },
                primary_action_id: "copy".into(),
                primary_action_label: "Copy".into(),
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
        if result.id.as_str() == "clip:clear" {
            return vec![ActionDescriptor {
                id: ActionId::new("clear"),
                label: "Clear unpinned".into(),
                risk: ActionRisk::Destructive,
                confirmation: true,
            }];
        }
        let Some(id) = result
            .id
            .as_str()
            .strip_prefix("clip:")
            .and_then(|s| s.parse::<i64>().ok())
        else {
            return Vec::new();
        };
        let pinned = self
            .store
            .get(id)
            .ok()
            .flatten()
            .map(|e| e.pinned)
            .unwrap_or(false);
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
                id: ActionId::new(if pinned { "unpin" } else { "pin" }),
                label: if pinned { "Unpin".into() } else { "Pin".into() },
                risk: ActionRisk::Safe,
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
        let id = action
            .result
            .id
            .as_str()
            .strip_prefix("clip:")
            .and_then(|s| s.parse::<i64>().ok());

        match action.action.id.as_str() {
            "copy" => {
                let Some(id) = id else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected clip:<id>".into(),
                        },
                    };
                };
                match self.store.get(id) {
                    Ok(Some(row)) => match self.pasteboard.write_text(&row.text).await {
                        Ok(()) => ActionOutcome::Success {
                            message: Some("copied".into()),
                        },
                        Err(err) => ActionOutcome::Failed {
                            kind: FailureKind::Unavailable {
                                reason: err.to_string(),
                                retryable: true,
                            },
                        },
                    },
                    Ok(None) => ActionOutcome::Failed {
                        kind: FailureKind::NotFound {
                            entity: format!("clip:{id}"),
                        },
                    },
                    Err(err) => ActionOutcome::Failed {
                        kind: FailureKind::Io {
                            context: err.to_string(),
                        },
                    },
                }
            }
            "paste" => {
                if !self.accessibility.is_trusted() {
                    return ActionOutcome::Failed {
                        kind: FailureKind::PermissionRequired {
                            capability: "accessibility".into(),
                            guidance: "Grant Accessibility to paste into other apps".into(),
                        },
                    };
                }
                let Some(id) = id else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected clip:<id>".into(),
                        },
                    };
                };
                match self.store.get(id) {
                    Ok(Some(row)) => match self.pasteboard.write_text(&row.text).await {
                        Ok(()) => match self.accessibility.paste_clipboard().await {
                            Ok(()) => ActionOutcome::Success {
                                message: Some("pasted".into()),
                            },
                            Err(_) => ActionOutcome::Failed {
                                kind: FailureKind::PermissionRequired {
                                    capability: "accessibility".into(),
                                    guidance: "Grant Accessibility to paste into other apps".into(),
                                },
                            },
                        },
                        Err(err) => ActionOutcome::Failed {
                            kind: FailureKind::Unavailable {
                                reason: err.to_string(),
                                retryable: true,
                            },
                        },
                    },
                    Ok(None) => ActionOutcome::Failed {
                        kind: FailureKind::NotFound {
                            entity: format!("clip:{id}"),
                        },
                    },
                    Err(err) => ActionOutcome::Failed {
                        kind: FailureKind::Io {
                            context: err.to_string(),
                        },
                    },
                }
            }
            "pin" => {
                let Some(id) = id else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected clip:<id>".into(),
                        },
                    };
                };
                match self.store.set_pinned(id, true) {
                    Ok(()) => match Self::refresh_index(self.store.as_ref(), &self.index).await {
                        Ok(()) => ActionOutcome::Success {
                            message: Some("pinned".into()),
                        },
                        Err(err) => ActionOutcome::Failed {
                            kind: FailureKind::Io {
                                context: err.to_string(),
                            },
                        },
                    },
                    Err(err) => ActionOutcome::Failed {
                        kind: FailureKind::Io {
                            context: err.to_string(),
                        },
                    },
                }
            }
            "unpin" => {
                let Some(id) = id else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected clip:<id>".into(),
                        },
                    };
                };
                match self.store.set_pinned(id, false) {
                    Ok(()) => match Self::refresh_index(self.store.as_ref(), &self.index).await {
                        Ok(()) => ActionOutcome::Success {
                            message: Some("unpinned".into()),
                        },
                        Err(err) => ActionOutcome::Failed {
                            kind: FailureKind::Io {
                                context: err.to_string(),
                            },
                        },
                    },
                    Err(err) => ActionOutcome::Failed {
                        kind: FailureKind::Io {
                            context: err.to_string(),
                        },
                    },
                }
            }
            "clear" => {
                if action.action.confirmation && !action.confirmation {
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "confirmation required".into(),
                        },
                    };
                }
                match self.store.clear_unpinned() {
                    Ok(n) => match Self::refresh_index(self.store.as_ref(), &self.index).await {
                        Ok(()) => ActionOutcome::Success {
                            message: Some(format!("cleared {n} unpinned item(s)")),
                        },
                        Err(err) => ActionOutcome::Failed {
                            kind: FailureKind::Io {
                                context: err.to_string(),
                            },
                        },
                    },
                    Err(err) => ActionOutcome::Failed {
                        kind: FailureKind::Io {
                            context: err.to_string(),
                        },
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
                let Some(id) = id else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected clip:<id>".into(),
                        },
                    };
                };
                match self.store.delete(id) {
                    Ok(()) => match Self::refresh_index(self.store.as_ref(), &self.index).await {
                        Ok(()) => ActionOutcome::Success {
                            message: Some("deleted".into()),
                        },
                        Err(err) => ActionOutcome::Failed {
                            kind: FailureKind::Io {
                                context: err.to_string(),
                            },
                        },
                    },
                    Err(err) => ActionOutcome::Failed {
                        kind: FailureKind::Io {
                            context: err.to_string(),
                        },
                    },
                }
            }
            "open" => ActionOutcome::Failed {
                kind: FailureKind::NotFound {
                    entity: "action:open".into(),
                },
            },
            other => ActionOutcome::Failed {
                kind: FailureKind::NotFound {
                    entity: format!("action:{other}"),
                },
            },
        }
    }

    async fn teardown(&self) {
        self.stop_poller().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use luma_application::{FakeAccessibility, MemoryClipboardHistory, PasteboardError};
    use tokio::sync::Mutex as TokioMutex;

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

    fn denied_ax() -> Arc<dyn AccessibilityPort> {
        Arc::new(FakeAccessibility {
            trusted: false,
            paste_ok: false,
        })
    }

    fn test_repo() -> Arc<dyn ClipboardHistoryRepository> {
        Arc::new(MemoryClipboardHistory::new())
    }

    #[tokio::test]
    async fn copy_writes_pasteboard() {
        let repo = test_repo();
        let id = repo.insert("hello clip", false).unwrap();
        let pb = Arc::new(MemPb(TokioMutex::new(None)));
        let m = ClipboardModule::with_deps(
            repo,
            pb.clone(),
            denied_ax(),
            Arc::new(ClipboardSuppression::new()),
        );
        let outcome = m
            .perform(
                ActionRequest {
                    result: SearchItem {
                        id: luma_domain::ResultId::new(format!("clip:{id}")),
                        module_id: ModuleId::new("luma.clipboard"),
                        title: "hello".into(),
                        subtitle: None,
                        kind: "clip".into(),
                        score: 1.0,
                        primary_action: ActionDescriptor {
                            id: ActionId::new("copy"),
                            label: "Copy".into(),
                            risk: ActionRisk::Safe,
                            confirmation: false,
                        },
                        secondary_actions: vec![],
                    },
                    action: ActionDescriptor {
                        id: ActionId::new("copy"),
                        label: "Copy".into(),
                        risk: ActionRisk::Safe,
                        confirmation: false,
                    },
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(outcome, ActionOutcome::Success { .. }));
        assert_eq!(pb.read_text().await.unwrap().as_deref(), Some("hello clip"));
    }

    #[tokio::test]
    async fn paste_denied_is_not_success() {
        let m = ClipboardModule::with_deps(
            test_repo(),
            Arc::new(MemPb(TokioMutex::new(None))),
            denied_ax(),
            Arc::new(ClipboardSuppression::new()),
        );
        let outcome = m
            .perform(
                ActionRequest {
                    result: SearchItem {
                        id: luma_domain::ResultId::new("clip:1"),
                        module_id: ModuleId::new("luma.clipboard"),
                        title: "x".into(),
                        subtitle: None,
                        kind: "clip".into(),
                        score: 1.0,
                        primary_action: ActionDescriptor {
                            id: ActionId::new("paste"),
                            label: "Paste".into(),
                            risk: ActionRisk::Confirm,
                            confirmation: false,
                        },
                        secondary_actions: vec![],
                    },
                    action: ActionDescriptor {
                        id: ActionId::new("paste"),
                        label: "Paste".into(),
                        risk: ActionRisk::Confirm,
                        confirmation: false,
                    },
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(
            outcome,
            ActionOutcome::Failed {
                kind: FailureKind::PermissionRequired { .. }
            }
        ));
    }

    #[tokio::test]
    async fn warmup_seeds_last_seen_without_capturing_pasteboard() {
        let repo = test_repo();
        let pb = Arc::new(MemPb(TokioMutex::new(Some("pre-existing-secret".into()))));
        let m = ClipboardModule::with_deps(
            repo.clone(),
            pb,
            denied_ax(),
            Arc::new(ClipboardSuppression::new()),
        );
        m.warmup(WarmupContext {
            cancel: CancellationToken::new(),
        })
        .await;
        assert_eq!(
            m.last_seen_text.lock().await.as_deref(),
            Some("pre-existing-secret")
        );
        assert!(
            repo.list_page(0, 10).unwrap().is_empty(),
            "warmup must not insert pasteboard contents into history"
        );
        m.teardown().await;
    }

    #[tokio::test]
    async fn poller_stops_on_teardown() {
        let pb = Arc::new(MemPb(TokioMutex::new(Some("poll-me".into()))));
        let m = ClipboardModule::with_deps(
            test_repo(),
            pb,
            denied_ax(),
            Arc::new(ClipboardSuppression::new()),
        );
        let cancel = CancellationToken::new();
        m.warmup(WarmupContext {
            cancel: cancel.clone(),
        })
        .await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        m.teardown().await;
        assert!(m.poll_handle.lock().await.is_none());
    }
}
