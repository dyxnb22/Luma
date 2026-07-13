use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, LumaModule, ModuleManifest, ModuleState, SearchMode, SearchSink,
    WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_platform_macos::{Accessibility, MacAccessibility, MacPasteboard, Pasteboard};
use luma_protocol::{Event, SearchItemDto};
use luma_storage::{looks_secret, ClipboardStore};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

pub struct ClipboardModule {
    manifest: ModuleManifest,
    store: Arc<ClipboardStore>,
    pasteboard: Arc<dyn Pasteboard>,
    accessibility: Arc<dyn Accessibility>,
    index: Arc<RwLock<Vec<luma_storage::ClipboardRow>>>,
    poll_cancel: Mutex<Option<CancellationToken>>,
    poll_handle: Mutex<Option<JoinHandle<()>>>,
}

impl ClipboardModule {
    /// Open LumaNext clipboard store. Fails loudly — never falls back to a temp DB.
    pub fn try_new() -> Result<Self, luma_storage::ClipboardStoreError> {
        let store = ClipboardStore::luma_next_default()?;
        Ok(Self::with_deps(
            Arc::new(store),
            Arc::new(MacPasteboard),
            Arc::new(MacAccessibility),
        ))
    }

    pub fn with_deps(
        store: Arc<ClipboardStore>,
        pasteboard: Arc<dyn Pasteboard>,
        accessibility: Arc<dyn Accessibility>,
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
            index: Arc::new(RwLock::new(Vec::new())),
            poll_cancel: Mutex::new(None),
            poll_handle: Mutex::new(None),
        }
    }

    async fn refresh_index(
        store: &ClipboardStore,
        index: &RwLock<Vec<luma_storage::ClipboardRow>>,
    ) -> Result<(), luma_storage::ClipboardStoreError> {
        *index.write().await = store.list_page(0, 500)?;
        Ok(())
    }

    async fn capture_once(
        store: &ClipboardStore,
        index: &RwLock<Vec<luma_storage::ClipboardRow>>,
        pasteboard: &dyn Pasteboard,
    ) {
        if let Ok(Some(text)) = pasteboard.read_text().await {
            if looks_secret(&text) {
                return;
            }
            if let Ok(page) = store.list_page(0, 1) {
                if page.first().is_some_and(|r| r.text == text) {
                    return;
                }
            }
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
        let token = cancel.clone();
        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = token.cancelled() => break,
                    _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                        Self::capture_once(&store, &index, pasteboard.as_ref()).await;
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

impl Default for ClipboardModule {
    fn default() -> Self {
        Self::try_new().expect("ClipboardModule::default requires writable LumaNext")
    }
}

#[async_trait]
impl LumaModule for ClipboardModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, ctx: WarmupContext) -> ModuleState {
        if !ctx.cancel.is_cancelled() {
            if let Err(err) = Self::refresh_index(&self.store, &self.index).await {
                return ModuleState::Failed(err.to_string());
            }
            Self::capture_once(&self.store, &self.index, self.pasteboard.as_ref()).await;
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

        if needle.is_empty() {
            let count = self.index.read().await.len();
            let row = SearchItemDto {
                id: "clip:open".into(),
                module_id: "luma.clipboard".into(),
                title: "Clipboard history".into(),
                subtitle: Some(format!("{count} entries")),
                kind: "open".into(),
                score: 1.0,
                primary_action_id: "open".into(),
                primary_action_label: "Open".into(),
            };
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![row],
                    removed_ids: vec![],
                })
                .await;
            return;
        }

        let limit = if matches!(query.scope, luma_domain::QueryScope::Global) {
            query.limit.min(3)
        } else {
            query.limit
        };
        let needle = needle.to_lowercase();
        let mut rows: Vec<_> = self
            .index
            .read()
            .await
            .iter()
            .filter(|entry| entry.text.to_lowercase().contains(&needle))
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
            upserts.push(SearchItemDto {
                id: format!("clip:{}", entry.id),
                module_id: "luma.clipboard".into(),
                title: preview,
                subtitle: None,
                kind: "clip".into(),
                score: 60.0,
                primary_action_id: "copy".into(),
                primary_action_label: "Copy".into(),
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
            ActionDescriptor {
                id: ActionId::new("pin"),
                label: "Pin".into(),
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
                    Ok(()) => match Self::refresh_index(&self.store, &self.index).await {
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
                    Ok(()) => match Self::refresh_index(&self.store, &self.index).await {
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
            "open" => ActionOutcome::Success {
                message: Some("opened clipboard".into()),
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
    use luma_platform_macos::{FakeAccessibility, PasteboardError};
    use luma_storage::ClipboardStore;
    use tempfile::tempdir;
    use tokio::sync::Mutex as TokioMutex;

    struct MemPb(TokioMutex<Option<String>>);

    #[async_trait]
    impl Pasteboard for MemPb {
        async fn read_text(&self) -> Result<Option<String>, PasteboardError> {
            Ok(self.0.lock().await.clone())
        }
        async fn write_text(&self, text: &str) -> Result<(), PasteboardError> {
            *self.0.lock().await = Some(text.into());
            Ok(())
        }
    }

    fn denied_ax() -> Arc<dyn Accessibility> {
        Arc::new(FakeAccessibility {
            trusted: false,
            paste_ok: false,
        })
    }

    #[tokio::test]
    async fn copy_writes_pasteboard() {
        let dir = tempdir().unwrap();
        let store = Arc::new(ClipboardStore::with_path(dir.path().join("c.sqlite")).unwrap());
        let id = store.insert("hello clip", false).unwrap();
        let pb = Arc::new(MemPb(TokioMutex::new(None)));
        let m = ClipboardModule::with_deps(store, pb.clone(), denied_ax());
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
        let dir = tempdir().unwrap();
        let store = Arc::new(ClipboardStore::with_path(dir.path().join("c.sqlite")).unwrap());
        let m =
            ClipboardModule::with_deps(store, Arc::new(MemPb(TokioMutex::new(None))), denied_ax());
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
    async fn poller_stops_on_teardown() {
        let dir = tempdir().unwrap();
        let store = Arc::new(ClipboardStore::with_path(dir.path().join("c.sqlite")).unwrap());
        let pb = Arc::new(MemPb(TokioMutex::new(Some("poll-me".into()))));
        let m = ClipboardModule::with_deps(store, pb, denied_ax());
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
