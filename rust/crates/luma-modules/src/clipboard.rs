use crate::cancel::await_unless_cancelled;
use async_trait::async_trait;
use luma_application::{
    looks_secret, paste_to_target_app, AccessibilityPort, ActionOutcome, ActionRequest,
    ClipboardEntry, ClipboardHistoryRepository, ClipboardRepoError, LumaModule, ModuleManifest,
    ModuleState, PasteboardPort, SearchMode, SearchSink, WarmupContext, WindowCatalogPort,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{Event, SearchItemDto};
use std::sync::atomic::{AtomicU64, Ordering};
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
    window_catalog: Arc<dyn WindowCatalogPort>,
    suppression: Arc<ClipboardSuppression>,
    retention_days: Arc<std::sync::atomic::AtomicU32>,
    last_seen_text: Arc<Mutex<Option<String>>>,
    index: Arc<RwLock<Vec<ClipboardEntry>>>,
    store_error: Arc<RwLock<Option<String>>>,
    /// Bumped on teardown so in-flight capture/refresh cannot resurrect caches.
    refresh_generation: Arc<AtomicU64>,
    poll_cancel: Mutex<Option<CancellationToken>>,
    poll_handle: Mutex<Option<JoinHandle<()>>>,
}

impl ClipboardModule {
    pub fn with_deps(
        store: Arc<dyn ClipboardHistoryRepository>,
        pasteboard: Arc<dyn PasteboardPort>,
        accessibility: Arc<dyn AccessibilityPort>,
        window_catalog: Arc<dyn WindowCatalogPort>,
        suppression: Arc<ClipboardSuppression>,
    ) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.clipboard"),
                display_name: "Clipboard".into(),
                triggers: vec!["clip".into(), "cb".into()],
                default_enabled: true,
                search_mode: SearchMode::GlobalContributing,
                // History/search/copy work without Accessibility. Paste reports its AX
                // requirement locally when the action is attempted.
                required_capabilities: vec![],
                workbench: luma_application::WorkbenchMeta {
                    glyph: Some("C".into()),
                    suggested_query: Some("/clip ".into()),
                    empty_hint: Some("/clip · history · pin/unpin · paste needs AX".into()),
                    supports_browse: false,
                },
            },
            store,
            pasteboard,
            accessibility,
            window_catalog,
            suppression,
            retention_days: Arc::new(std::sync::atomic::AtomicU32::new(DEFAULT_RETENTION_DAYS)),
            last_seen_text: Arc::new(Mutex::new(None)),
            index: Arc::new(RwLock::new(Vec::new())),
            store_error: Arc::new(RwLock::new(None)),
            refresh_generation: Arc::new(AtomicU64::new(0)),
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
        store_error: &RwLock<Option<String>>,
        generation: u64,
        refresh_generation: &AtomicU64,
    ) -> Result<(), ClipboardRepoError> {
        if refresh_generation.load(Ordering::SeqCst) != generation {
            return Ok(());
        }
        match store.list_page(0, 64) {
            Ok(rows) => {
                if refresh_generation.load(Ordering::SeqCst) != generation {
                    return Ok(());
                }
                *index.write().await = rows;
                *store_error.write().await = None;
                Ok(())
            }
            Err(err) => {
                if refresh_generation.load(Ordering::SeqCst) == generation {
                    *store_error.write().await = Some(err.to_string());
                }
                Err(err)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn capture_once(
        store: &dyn ClipboardHistoryRepository,
        index: &RwLock<Vec<ClipboardEntry>>,
        store_error: &RwLock<Option<String>>,
        pasteboard: &dyn PasteboardPort,
        suppression: &ClipboardSuppression,
        last_seen_text: &Mutex<Option<String>>,
        retention_days: u32,
        generation: u64,
        refresh_generation: &AtomicU64,
    ) {
        if refresh_generation.load(Ordering::SeqCst) != generation {
            return;
        }
        if let Ok(Some(text)) = pasteboard.read_text().await {
            if refresh_generation.load(Ordering::SeqCst) != generation {
                return;
            }
            if looks_secret(&text) || suppression.is_suppressed(&text) {
                let mut last = last_seen_text.lock().await;
                if refresh_generation.load(Ordering::SeqCst) == generation {
                    *last = Some(text);
                }
                return;
            }
            {
                let mut last = last_seen_text.lock().await;
                if last.as_ref() == Some(&text) {
                    return;
                }
                if refresh_generation.load(Ordering::SeqCst) != generation {
                    return;
                }
                *last = Some(text.clone());
            }
            if let Ok(Some(latest)) = store.latest_by_created() {
                if latest.text == text {
                    return;
                }
            }
            if let Err(err) = store.purge_older_than_days(retention_days) {
                if refresh_generation.load(Ordering::SeqCst) == generation {
                    *store_error.write().await = Some(format!("purge failed: {err}"));
                }
            }
            match store.insert(&text, false) {
                Ok(_) => {
                    let _ = Self::refresh_index(
                        store,
                        index,
                        store_error,
                        generation,
                        refresh_generation,
                    )
                    .await;
                }
                Err(err) => {
                    if refresh_generation.load(Ordering::SeqCst) == generation {
                        *store_error.write().await = Some(format!("insert failed: {err}"));
                    }
                }
            }
        }
    }

    async fn start_poller(&self, parent: CancellationToken) {
        self.stop_poller().await;
        let cancel = parent.child_token();
        let store = self.store.clone();
        let index = self.index.clone();
        let store_error = self.store_error.clone();
        let pasteboard = self.pasteboard.clone();
        let suppression = self.suppression.clone();
        let last_seen_text = self.last_seen_text.clone();
        let retention_days = self.retention_days.clone();
        let refresh_generation = self.refresh_generation.clone();
        let generation = refresh_generation.load(Ordering::SeqCst);
        let token = cancel.clone();
        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = token.cancelled() => break,
                    _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                        let days = retention_days.load(std::sync::atomic::Ordering::Relaxed);
                        Self::capture_once(
                            store.as_ref(),
                            &index,
                            &store_error,
                            pasteboard.as_ref(),
                            &suppression,
                            &last_seen_text,
                            days,
                            generation,
                            &refresh_generation,
                        ).await;
                    }
                }
            }
        });
        *self.poll_cancel.lock().await = Some(cancel);
        *self.poll_handle.lock().await = Some(handle);
    }

    async fn refresh_index_now(&self) -> Result<(), ClipboardRepoError> {
        let generation = self.refresh_generation.load(Ordering::SeqCst);
        Self::refresh_index(
            self.store.as_ref(),
            &self.index,
            &self.store_error,
            generation,
            &self.refresh_generation,
        )
        .await
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
            if let Err(err) = self.refresh_index_now().await {
                return ModuleState::Failed(err.to_string());
            }
            // Seed last_seen from the current pasteboard without capturing into history.
            // Avoids writing a pre-existing secret (e.g. after Secrets copy + restart).
            if let Ok(Some(text)) = self.pasteboard.read_text().await {
                *self.last_seen_text.lock().await = Some(text);
            }
            let days = self
                .retention_days
                .load(std::sync::atomic::Ordering::Relaxed);
            if let Err(err) = self.store.purge_older_than_days(days) {
                *self.store_error.write().await = Some(format!("purge failed: {err}"));
                return ModuleState::Failed(err.to_string());
            }
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

        if let Some(err) = self.store_error.read().await.clone() {
            if matches!(query.scope, luma_domain::QueryScope::Targeted { .. }) {
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![SearchItemDto {
                            id: "clip:unavailable".into(),
                            module_id: "luma.clipboard".into(),
                            title: "Clipboard store unavailable".into(),
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
            }
            return;
        }

        let rows = match tokio::task::spawn_blocking({
            let store = self.store.clone();
            let needle = needle.clone();
            move || {
                if needle.is_empty() {
                    store.list_page(0, limit)
                } else {
                    store.search_text(&needle, limit)
                }
            }
        })
        .await
        {
            Ok(Ok(rows)) => rows,
            Ok(Err(err)) => {
                if matches!(query.scope, luma_domain::QueryScope::Targeted { .. }) {
                    let _ = sink
                        .send(Event::ResultsChunk {
                            request_id: String::new(),
                            sequence: 1,
                            upserts: vec![SearchItemDto {
                                id: "clip:unavailable".into(),
                                module_id: "luma.clipboard".into(),
                                title: "Clipboard store unavailable".into(),
                                subtitle: Some(crate::ux::friendly_store_error(&err.to_string())),
                                kind: "unavailable".into(),
                                score: 0.0,
                                primary_action_id: "noop".into(),
                                primary_action_label: "Unavailable".into(),
                                ..Default::default()
                            }],
                            removed_ids: vec![],
                        })
                        .await;
                }
                return;
            }
            _ => return,
        };
        let rows: Vec<_> = rows
            .into_iter()
            .filter(|entry| !looks_secret(&entry.text))
            .collect();
        let mut upserts = Vec::new();
        if upserts.is_empty()
            && rows.is_empty()
            && needle.is_empty()
            && matches!(query.scope, luma_domain::QueryScope::Targeted { .. })
        {
            upserts.push(SearchItemDto {
                id: "clip:empty".into(),
                module_id: "luma.clipboard".into(),
                title: "Clipboard history is empty".into(),
                subtitle: Some(
                    "Copy text elsewhere — new clips appear here; pin to keep across clear/purge"
                        .into(),
                ),
                kind: "onboarding".into(),
                score: 0.0,
                primary_action_id: "noop".into(),
                primary_action_label: "OK".into(),
                ..Default::default()
            });
        } else if upserts.is_empty()
            && rows.is_empty()
            && !needle.is_empty()
            && matches!(query.scope, luma_domain::QueryScope::Targeted { .. })
        {
            upserts.push(SearchItemDto {
                id: "clip:no-matches".into(),
                module_id: "luma.clipboard".into(),
                title: format!("No clips matching \"{needle}\""),
                subtitle: Some("Try another query · /clip clear".into()),
                kind: "status".into(),
                score: 0.0,
                primary_action_id: "noop".into(),
                primary_action_label: "OK".into(),
                ..Default::default()
            });
        }
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
        if result.id.as_str() == "clip:empty"
            || result.id.as_str() == "clip:no-matches"
            || result.id.as_str() == "clip:unavailable"
            || result.kind == "onboarding"
            || result.kind == "status"
            || result.kind == "unavailable"
        {
            return vec![ActionDescriptor {
                id: ActionId::new("noop"),
                label: "OK".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
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

    async fn preview(&self, result: &SearchItem) -> Option<String> {
        let id = result
            .id
            .as_str()
            .strip_prefix("clip:")
            .and_then(|s| s.parse::<i64>().ok())?;
        let entry = self.store.get(id).ok().flatten()?;
        if looks_secret(&entry.text) {
            return Some("[redacted — matched secret heuristic]".into());
        }
        const PREVIEW_LIMIT: usize = 12_000;
        let body: String = entry.text.chars().take(PREVIEW_LIMIT).collect();
        if entry.text.chars().count() > PREVIEW_LIMIT {
            Some(format!("{body}\n…"))
        } else {
            Some(body)
        }
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
            "noop" => ActionOutcome::Success { message: None },
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
                    Ok(Some(row)) => {
                        match await_unless_cancelled(&cancel, self.pasteboard.write_text(&row.text))
                            .await
                        {
                            None => ActionOutcome::Cancelled,
                            Some(Ok(())) => {
                                *self.last_seen_text.lock().await = Some(row.text.clone());
                                self.suppression
                                    .suppress(&row.text, std::time::Duration::from_secs(45));
                                ActionOutcome::Success {
                                    message: Some("copied".into()),
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
                let Some(id) = id else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected clip:<id>".into(),
                        },
                    };
                };
                match self.store.get(id) {
                    Ok(Some(row)) => {
                        *self.last_seen_text.lock().await = Some(row.text.clone());
                        self.suppression
                            .suppress(&row.text, std::time::Duration::from_secs(45));
                        match await_unless_cancelled(
                            &cancel,
                            paste_to_target_app(
                                self.window_catalog.clone(),
                                self.pasteboard.clone(),
                                self.accessibility.clone(),
                                &row.text,
                            ),
                        )
                        .await
                        {
                            None => ActionOutcome::Cancelled,
                            Some(Ok(())) => ActionOutcome::Success {
                                message: Some("pasted".into()),
                            },
                            Some(Err(kind)) => ActionOutcome::Failed { kind },
                        }
                    }
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
                    Ok(()) => match self.refresh_index_now().await {
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
                    Ok(()) => match self.refresh_index_now().await {
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
                    Ok(n) => match self.refresh_index_now().await {
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
                    Ok(()) => match self.refresh_index_now().await {
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
        self.refresh_generation.fetch_add(1, Ordering::SeqCst);
        self.stop_poller().await;
        *self.index.write().await = Vec::new();
        *self.store_error.write().await = None;
        *self.last_seen_text.lock().await = None;
    }

    async fn apply_settings(&self, settings: &luma_application::AppSettings) {
        let days = settings.clipboard_retention_days.max(1);
        self.retention_days
            .store(days, std::sync::atomic::Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use luma_application::{
        FakeAccessibility, FakeWindowCatalog, MemoryClipboardHistory, PasteboardError,
    };
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

    fn denied_ax() -> Arc<dyn AccessibilityPort> {
        Arc::new(FakeAccessibility::new(false, false))
    }

    fn test_catalog() -> Arc<dyn WindowCatalogPort> {
        Arc::new(FakeWindowCatalog::with_entries(
            vec![],
            Some("Safari".into()),
        ))
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
            test_catalog(),
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
                        ui_intent: None,
                        action_payload: None,
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
    async fn clipboard_copy_cancelled_does_not_write_pasteboard() {
        let repo = test_repo();
        let id = repo.insert("hello clip", false).unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel();
        let pb = Arc::new(HangPb {
            written: TokioMutex::new(None),
            started: tokio::sync::Notify::new(),
            release: TokioMutex::new(Some(rx)),
        });
        let m = ClipboardModule::with_deps(
            repo,
            pb.clone(),
            denied_ax(),
            test_catalog(),
            Arc::new(ClipboardSuppression::new()),
        );
        let cancel = CancellationToken::new();
        let cancel_c = cancel.clone();
        let started = pb.started.notified();
        tokio::pin!(started);
        let perform = tokio::spawn(async move {
            m.perform(
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
                        ui_intent: None,
                        action_payload: None,
                    },
                    action: ActionDescriptor {
                        id: ActionId::new("copy"),
                        label: "Copy".into(),
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

    #[tokio::test]
    async fn paste_denied_is_not_success() {
        let repo = test_repo();
        let id = repo.insert("secret text", false).unwrap();
        let m = ClipboardModule::with_deps(
            repo,
            Arc::new(MemPb(TokioMutex::new(None))),
            denied_ax(),
            test_catalog(),
            Arc::new(ClipboardSuppression::new()),
        );
        let outcome = m
            .perform(
                ActionRequest {
                    result: SearchItem {
                        id: luma_domain::ResultId::new(format!("clip:{id}")),
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
                        ui_intent: None,
                        action_payload: None,
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
    async fn paste_without_target_is_unavailable_with_remediation() {
        let repo = test_repo();
        let id = repo.insert("paste me", false).unwrap();
        let catalog = Arc::new(FakeWindowCatalog::default());
        let m = ClipboardModule::with_deps(
            repo,
            Arc::new(MemPb(TokioMutex::new(None))),
            Arc::new(FakeAccessibility::new(true, true)),
            catalog.clone(),
            Arc::new(ClipboardSuppression::new()),
        );
        let outcome = m
            .perform(
                ActionRequest {
                    result: SearchItem {
                        id: luma_domain::ResultId::new(format!("clip:{id}")),
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
                        ui_intent: None,
                        action_payload: None,
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
        match outcome {
            ActionOutcome::Failed {
                kind: FailureKind::Unavailable { reason, .. },
            } => {
                assert!(
                    reason.contains("no paste target"),
                    "unexpected reason: {reason}"
                );
                assert!(
                    reason.contains("Hub") || reason.contains("win"),
                    "reason should mention Hub/win remediation: {reason}"
                );
            }
            other => panic!("expected Unavailable, got {other:?}"),
        }
        assert!(
            catalog.focus_app_calls.lock().await.is_empty(),
            "must not focus when paste target is missing"
        );
    }

    #[tokio::test]
    async fn warmup_seeds_last_seen_without_capturing_pasteboard() {
        let repo = test_repo();
        let pb = Arc::new(MemPb(TokioMutex::new(Some("pre-existing-secret".into()))));
        let m = ClipboardModule::with_deps(
            repo.clone(),
            pb,
            denied_ax(),
            test_catalog(),
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
    async fn teardown_releases_runtime_caches() {
        let repo = test_repo();
        repo.insert("cached clip", false).unwrap();
        let m = ClipboardModule::with_deps(
            repo,
            Arc::new(MemPb(TokioMutex::new(None))),
            denied_ax(),
            test_catalog(),
            Arc::new(ClipboardSuppression::new()),
        );
        m.warmup(WarmupContext {
            cancel: CancellationToken::new(),
        })
        .await;
        assert!(!m.index.read().await.is_empty());
        m.teardown().await;
        assert!(m.index.read().await.is_empty());
        assert!(m.store_error.read().await.is_none());
        assert!(m.last_seen_text.lock().await.is_none());
        m.teardown().await;
    }

    #[tokio::test]
    async fn poller_stops_on_teardown() {
        let pb = Arc::new(MemPb(TokioMutex::new(Some("poll-me".into()))));
        let m = ClipboardModule::with_deps(
            test_repo(),
            pb,
            denied_ax(),
            test_catalog(),
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

    #[tokio::test]
    async fn store_error_emits_unavailable_not_empty() {
        struct FailingStore;
        #[async_trait]
        impl ClipboardHistoryRepository for FailingStore {
            fn list_page(
                &self,
                _offset: usize,
                _limit: usize,
            ) -> Result<Vec<ClipboardEntry>, ClipboardRepoError> {
                Err(ClipboardRepoError::msg("disk full"))
            }
            fn search_text(
                &self,
                _needle: &str,
                _limit: usize,
            ) -> Result<Vec<ClipboardEntry>, ClipboardRepoError> {
                Err(ClipboardRepoError::msg("disk full"))
            }
            fn get(&self, _id: i64) -> Result<Option<ClipboardEntry>, ClipboardRepoError> {
                Err(ClipboardRepoError::msg("disk full"))
            }
            fn latest_by_created(&self) -> Result<Option<ClipboardEntry>, ClipboardRepoError> {
                Err(ClipboardRepoError::msg("disk full"))
            }
            fn insert(&self, _text: &str, _pinned: bool) -> Result<i64, ClipboardRepoError> {
                Err(ClipboardRepoError::msg("disk full"))
            }
            fn set_pinned(&self, _id: i64, _pinned: bool) -> Result<(), ClipboardRepoError> {
                Err(ClipboardRepoError::msg("disk full"))
            }
            fn delete(&self, _id: i64) -> Result<(), ClipboardRepoError> {
                Err(ClipboardRepoError::msg("disk full"))
            }
            fn clear_unpinned(&self) -> Result<usize, ClipboardRepoError> {
                Err(ClipboardRepoError::msg("disk full"))
            }
            fn purge_older_than_days(&self, _days: u32) -> Result<usize, ClipboardRepoError> {
                Err(ClipboardRepoError::msg("disk full"))
            }
        }

        let m = ClipboardModule::with_deps(
            Arc::new(FailingStore),
            Arc::new(MemPb(TokioMutex::new(None))),
            denied_ax(),
            test_catalog(),
            Arc::new(ClipboardSuppression::new()),
        );
        let state = m
            .warmup(WarmupContext {
                cancel: CancellationToken::new(),
            })
            .await;
        assert!(matches!(state, ModuleState::Failed(_)), "{state:?}");

        let (tx, mut rx) = tokio::sync::mpsc::channel(8);
        m.search(Query::parse("clip ", 50), tx, CancellationToken::new())
            .await;
        let ev = rx.recv().await.expect("chunk");
        match ev {
            Event::ResultsChunk { upserts, .. } => {
                assert_eq!(upserts.len(), 1);
                assert_eq!(upserts[0].id, "clip:unavailable");
                assert_eq!(upserts[0].kind, "unavailable");
            }
            other => panic!("unexpected {other:?}"),
        }
    }
}
