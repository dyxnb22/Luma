use crate::module::{LumaModule, ModuleState, WarmupContext};
use crate::port::EnginePort;
use crate::registry::ModuleRegistry;
use async_trait::async_trait;
use luma_domain::{Query, QueryScope};
use luma_protocol::{Command, Event, SearchItemDto};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio::task::{JoinHandle, JoinSet};
use tokio_util::sync::CancellationToken;
use tracing::warn;

const SEARCH_CANCEL_BOUND: Duration = Duration::from_millis(750);

fn is_meta_prefix(token: &str) -> bool {
    matches!(token, "doctor" | "help")
}

struct SearchTask {
    cancel: CancellationToken,
    module_cancels: HashMap<String, CancellationToken>,
    /// Owns module search tasks + result collector; cancel awaits this with a bound.
    handle: JoinHandle<()>,
}

struct OperationTask {
    cancel: CancellationToken,
    module_id: String,
    handle: Option<JoinHandle<()>>,
}

struct EngineInner {
    registry: ModuleRegistry,
    event_tx: mpsc::Sender<Event>,
    event_broadcast_tx: broadcast::Sender<Event>,
    session_cancel: CancellationToken,
    searches: HashMap<String, SearchTask>,
    /// Cancel arrived before Search registered — Search must abort on start.
    cancel_intents: HashMap<String, ()>,
    /// Search registered under lifecycle but not yet promoted to `searches`.
    pending_searches: HashMap<String, CancellationToken>,
    operations: HashMap<String, OperationTask>,
    results_by_id: HashMap<String, luma_domain::SearchItem>,
}

/// Optional infrastructure injected at composition time.
#[derive(Default)]
pub struct EngineOptions {
    pub settings: Option<Arc<dyn crate::ports::SettingsRepository>>,
    pub diagnostics: Option<Arc<dyn crate::ports::DiagnosticsSink>>,
    /// Modules skipped at composition (id, reason) — surfaced in Doctor.
    pub skipped_modules: Vec<(String, String)>,
}

/// In-process engine: owns modules, searches, and operations.
pub struct Engine {
    inner: Arc<Mutex<EngineInner>>,
    event_rx_slot: Mutex<Option<mpsc::Receiver<Event>>>,
    event_broadcast_tx: broadcast::Sender<Event>,
    settings: Option<Arc<dyn crate::ports::SettingsRepository>>,
    diagnostics: Option<Arc<dyn crate::ports::DiagnosticsSink>>,
    skipped_modules: Vec<(String, String)>,
    /// Serializes search setup so cancel→clear→register cannot interleave.
    search_lifecycle: Mutex<()>,
}

impl Engine {
    pub fn new(registry: ModuleRegistry) -> Self {
        Self::with_options(registry, EngineOptions::default())
    }

    pub fn with_settings(
        registry: ModuleRegistry,
        settings: Option<Arc<dyn crate::ports::SettingsRepository>>,
    ) -> Self {
        Self::with_options(
            registry,
            EngineOptions {
                settings,
                diagnostics: None,
                skipped_modules: Vec::new(),
            },
        )
    }

    pub fn with_options(registry: ModuleRegistry, options: EngineOptions) -> Self {
        let (event_tx, event_rx) = mpsc::channel(256);
        let (event_broadcast_tx, _) = broadcast::channel(256);
        Self {
            inner: Arc::new(Mutex::new(EngineInner {
                registry,
                event_tx,
                event_broadcast_tx: event_broadcast_tx.clone(),
                session_cancel: CancellationToken::new(),
                searches: HashMap::new(),
                cancel_intents: HashMap::new(),
                pending_searches: HashMap::new(),
                operations: HashMap::new(),
                results_by_id: HashMap::new(),
            })),
            event_rx_slot: Mutex::new(Some(event_rx)),
            event_broadcast_tx,
            settings: options.settings,
            diagnostics: options.diagnostics,
            skipped_modules: options.skipped_modules,
            search_lifecycle: Mutex::new(()),
        }
    }

    /// Backward-compatible alias used by older call sites/tests.
    pub fn with_config(
        registry: ModuleRegistry,
        config: Option<Arc<luma_storage::ConfigStore>>,
    ) -> Self {
        Self::with_settings(
            registry,
            config.map(|store| {
                Arc::new(crate::adapters::TomlSettingsRepository::new(store))
                    as Arc<dyn crate::ports::SettingsRepository>
            }),
        )
    }

    /// Take the unique event receiver (once). Used by TUI and one-shot CLI helpers.
    pub async fn take_event_receiver(&self) -> Option<mpsc::Receiver<Event>> {
        self.event_rx_slot.lock().await.take()
    }

    pub async fn start_session(&self) {
        let (enabled, disabled_ids) = {
            let g = self.inner.lock().await;
            let enabled = g.registry.enabled_modules();
            let disabled_ids: Vec<String> = g
                .registry
                .list()
                .into_iter()
                .filter(|(_, enabled, _)| !*enabled)
                .map(|(id, _, _)| id)
                .collect();
            (enabled, disabled_ids)
        };
        for id in disabled_ids {
            let _ = self
                .emit(Event::ModuleStateChanged {
                    module_id: id,
                    state: "disabled".into(),
                })
                .await;
        }

        let mut set = tokio::task::JoinSet::new();
        for module in enabled {
            let cancel = {
                let g = self.inner.lock().await;
                g.session_cancel.child_token()
            };
            set.spawn(async move {
                let id = module.manifest().id.as_str().to_string();
                let state = module
                    .warmup(WarmupContext {
                        cancel: cancel.clone(),
                    })
                    .await;
                (id, state)
            });
        }
        while let Some(joined) = set.join_next().await {
            match joined {
                Ok((id, state)) => {
                    let _ = self
                        .emit(Event::ModuleStateChanged {
                            module_id: id,
                            state: match state {
                                ModuleState::Ready => "ready".into(),
                                ModuleState::Cold => "cold".into(),
                                ModuleState::Disabled => "disabled".into(),
                                ModuleState::Failed(msg) => format!("failed:{msg}"),
                            },
                        })
                        .await;
                }
                Err(err) => {
                    warn!(?err, "module warmup task panicked");
                }
            }
        }
        let modules = {
            let g = self.inner.lock().await;
            g.registry.list_module_info().into_iter().collect()
        };
        let _ = self.emit(Event::SessionReady { modules }).await;
    }

    /// Cancel in-flight search/action work for a module, await operation termination, then teardown or warmup.
    async fn apply_module_enabled(&self, module_id: &str, enabled: bool) -> bool {
        let (module, op_handles) = {
            let mut g = self.inner.lock().await;
            if !g.registry.set_enabled(module_id, enabled) {
                return false;
            }
            let mut op_handles = Vec::new();
            if !enabled {
                for task in g.searches.values() {
                    if let Some(token) = task.module_cancels.get(module_id) {
                        token.cancel();
                    }
                }
                for op in g.operations.values_mut() {
                    if op.module_id == module_id {
                        op.cancel.cancel();
                        if let Some(handle) = op.handle.take() {
                            op_handles.push(handle);
                        }
                    }
                }
            }
            (g.registry.get(module_id), op_handles)
        };
        for handle in op_handles {
            let _ = handle.await;
        }
        let Some(module) = module else {
            return false;
        };
        if enabled {
            let cancel = {
                let g = self.inner.lock().await;
                g.session_cancel.child_token()
            };
            let state = module
                .warmup(WarmupContext {
                    cancel: cancel.clone(),
                })
                .await;
            let _ = self
                .emit(Event::ModuleStateChanged {
                    module_id: module_id.to_string(),
                    state: match state {
                        ModuleState::Ready => "ready".into(),
                        ModuleState::Cold => "cold".into(),
                        ModuleState::Disabled => "disabled".into(),
                        ModuleState::Failed(msg) => format!("failed:{msg}"),
                    },
                })
                .await;
        } else {
            module.teardown().await;
            let _ = self
                .emit(Event::ModuleStateChanged {
                    module_id: module_id.to_string(),
                    state: "disabled".into(),
                })
                .await;
        }
        true
    }

    async fn emit(&self, event: Event) -> Result<(), String> {
        let (tx, broadcast_tx) = {
            let g = self.inner.lock().await;
            (g.event_tx.clone(), g.event_broadcast_tx.clone())
        };
        let _ = broadcast_tx.send(event.clone());
        tx.send(event).await.map_err(|e| e.to_string())
    }

    /// Signal cancel, then bounded-await the search supervisor (modules + collector).
    async fn cancel_search_task(task: SearchTask) {
        task.cancel.cancel();
        let abort = task.handle.abort_handle();
        match tokio::time::timeout(SEARCH_CANCEL_BOUND, task.handle).await {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                if !err.is_cancelled() {
                    warn!(?err, "search supervisor ended with error during cancel");
                }
            }
            Err(_) => {
                abort.abort();
            }
        }
    }

    /// Cancel one search under `search_lifecycle`. Emits `SearchCancelled` exactly once when
    /// the request was known (running, pending, or pre-registered intent).
    async fn cancel_search(&self, request_id: &str) {
        let _lifecycle = self.search_lifecycle.lock().await;
        self.cancel_search_locked(request_id).await;
    }

    async fn cancel_search_locked(&self, request_id: &str) {
        if let Some(cancel) = {
            let mut g = self.inner.lock().await;
            g.pending_searches.remove(request_id)
        } {
            cancel.cancel();
            let _ = self
                .emit(Event::SearchCancelled {
                    request_id: request_id.to_string(),
                })
                .await;
            return;
        }

        let task = {
            let mut g = self.inner.lock().await;
            g.searches.remove(request_id)
        };
        if let Some(task) = task {
            Self::cancel_search_task(task).await;
            let _ = self
                .emit(Event::SearchCancelled {
                    request_id: request_id.to_string(),
                })
                .await;
            return;
        }

        // Search not registered yet — remember so a racing handle_search aborts,
        // and emit now so clients are not left without a terminal event.
        {
            let mut g = self.inner.lock().await;
            g.cancel_intents.insert(request_id.to_string(), ());
        }
        let _ = self
            .emit(Event::SearchCancelled {
                request_id: request_id.to_string(),
            })
            .await;
    }

    /// Cancel every running search and emit one `SearchCancelled` per request.
    /// Caller must hold `search_lifecycle`.
    async fn cancel_all_searches_locked(&self) {
        let tasks = {
            let mut g = self.inner.lock().await;
            g.searches.drain().collect::<Vec<_>>()
        };
        for (request_id, task) in tasks {
            Self::cancel_search_task(task).await;
            let _ = self
                .emit(Event::SearchCancelled {
                    request_id: request_id.clone(),
                })
                .await;
        }
        let pending = {
            let mut g = self.inner.lock().await;
            g.pending_searches.drain().collect::<Vec<_>>()
        };
        for (request_id, cancel) in pending {
            cancel.cancel();
            let _ = self.emit(Event::SearchCancelled { request_id }).await;
        }
    }

    async fn handle_search(&self, request_id: String, query_raw: String) {
        let _lifecycle = self.search_lifecycle.lock().await;

        // Cancel-before-registration: honor intent (SearchCancelled already emitted).
        let pre_cancelled = {
            let mut g = self.inner.lock().await;
            g.cancel_intents.remove(&request_id).is_some()
        };
        if pre_cancelled {
            return;
        }

        self.cancel_all_searches_locked().await;
        {
            let mut g = self.inner.lock().await;
            g.results_by_id.clear();
        }

        let cancel = {
            let g = self.inner.lock().await;
            g.session_cancel.child_token()
        };
        {
            let mut g = self.inner.lock().await;
            g.pending_searches
                .insert(request_id.clone(), cancel.clone());
        }

        // Intent recorded while we held lifecycle is impossible; token cancel means
        // cancel_search_locked already emitted for this pending id.
        if cancel.is_cancelled() {
            let mut g = self.inner.lock().await;
            g.pending_searches.remove(&request_id);
            return;
        }

        let _ = self
            .emit(Event::SearchStarted {
                request_id: request_id.clone(),
            })
            .await;
        let _ = self
            .emit(Event::ResultsReset {
                request_id: request_id.clone(),
            })
            .await;

        let query = {
            let g = self.inner.lock().await;
            let triggers = g.registry.all_triggers();
            Query::parse_with_prefixes(query_raw, 50, |token| {
                is_meta_prefix(token) || triggers.iter().any(|t| t == token)
            })
        };
        let modules: Vec<Arc<dyn LumaModule>> = {
            let g = self.inner.lock().await;
            match &query.scope {
                QueryScope::Targeted { module } => {
                    g.registry.resolve_trigger(module).into_iter().collect()
                }
                QueryScope::Global => g.registry.contributing(),
            }
        };

        if modules.is_empty() {
            {
                let mut g = self.inner.lock().await;
                g.pending_searches.remove(&request_id);
            }
            let _ = self
                .emit(Event::SearchFinished {
                    request_id,
                    total: 0,
                    elapsed_ms: 0,
                })
                .await;
            return;
        }

        if cancel.is_cancelled() {
            {
                let mut g = self.inner.lock().await;
                g.pending_searches.remove(&request_id);
            }
            // cancel_search_locked already emitted SearchCancelled for pending.
            return;
        }

        let (chunk_tx, mut chunk_rx) = mpsc::channel::<Event>(64);
        let engine = self.clone_inner();
        let request_for_task = request_id.clone();
        let cancel_for_task = cancel.clone();

        let mut module_cancels = HashMap::new();
        let mut set = JoinSet::new();
        for module in modules {
            let q = query.clone();
            let sink = chunk_tx.clone();
            let module_id = module.manifest().id.as_str().to_string();
            let token = cancel_for_task.child_token();
            module_cancels.insert(module_id, token.clone());
            set.spawn(async move {
                module.search(q, sink, token).await;
            });
        }
        drop(chunk_tx);

        set.spawn({
            let request_id = request_id.clone();
            let engine = engine.clone();
            let cancel_for_collect = cancel_for_task.clone();
            async move {
                let mut sequence = 0u64;
                let mut total = 0usize;
                while let Some(ev) = chunk_rx.recv().await {
                    if cancel_for_collect.is_cancelled() {
                        break;
                    }
                    if let Event::ResultsChunk {
                        upserts,
                        removed_ids,
                        ..
                    } = ev
                    {
                        sequence += 1;
                        total += upserts.len();
                        {
                            let mut g = engine.lock().await;
                            for u in &upserts {
                                g.results_by_id
                                    .insert(u.id.clone(), u.clone().into_domain());
                            }
                            for id in &removed_ids {
                                g.results_by_id.remove(id);
                            }
                        }
                        let (tx, broadcast_tx) = {
                            let g = engine.lock().await;
                            (g.event_tx.clone(), g.event_broadcast_tx.clone())
                        };
                        let event = Event::ResultsChunk {
                            request_id: request_id.clone(),
                            sequence,
                            upserts,
                            removed_ids,
                        };
                        let _ = broadcast_tx.send(event.clone());
                        let _ = tx.send(event).await;
                    }
                }
                // Terminal cancel is emitted by cancel_search_* (exactly once).
                // Collector only emits SearchFinished on clean completion.
                if !cancel_for_collect.is_cancelled() {
                    let (tx, broadcast_tx) = {
                        let g = engine.lock().await;
                        (g.event_tx.clone(), g.event_broadcast_tx.clone())
                    };
                    let event = Event::SearchFinished {
                        request_id,
                        total,
                        elapsed_ms: 0,
                    };
                    let _ = broadcast_tx.send(event.clone());
                    let _ = tx.send(event).await;
                }
            }
        });

        // Aborting this supervisor drops JoinSet, which aborts every owned child.
        // Also abort_all on cancel so non-cooperative modules cannot outlive cancel.
        let supervisor = tokio::spawn(async move {
            tokio::select! {
                _ = cancel_for_task.cancelled() => {
                    set.abort_all();
                    while let Some(joined) = set.join_next().await {
                        if let Err(err) = joined {
                            if !err.is_cancelled() {
                                warn!(?err, "search JoinSet task ended with error after abort");
                            }
                        }
                    }
                }
                _ = async {
                    while let Some(joined) = set.join_next().await {
                        if let Err(err) = joined {
                            if !err.is_cancelled() {
                                warn!(?err, "search JoinSet task ended with error");
                            }
                        }
                    }
                } => {}
            }
        });

        {
            let mut g = self.inner.lock().await;
            g.pending_searches.remove(&request_for_task);
            g.searches.insert(
                request_for_task,
                SearchTask {
                    cancel,
                    module_cancels,
                    handle: supervisor,
                },
            );
        }
    }

    fn clone_inner(&self) -> Arc<Mutex<EngineInner>> {
        self.inner.clone()
    }

    pub async fn handle_command(&self, command: Command) {
        match command {
            Command::StartSession => self.start_session().await,
            Command::Search { request_id, query } => {
                self.handle_search(request_id, query).await;
            }
            Command::CancelSearch { request_id } => {
                self.cancel_search(&request_id).await;
            }
            Command::RunDoctor => {
                let (rows, settings_snapshot) = {
                    let g = self.inner.lock().await;
                    let rows = g.registry.list();
                    let settings_snapshot = self
                        .settings
                        .as_ref()
                        .and_then(|s| s.load_or_default().ok());
                    (rows, settings_snapshot)
                };
                let modules = rows
                    .iter()
                    .map(|(id, enabled, name)| {
                        serde_json::json!({
                            "id": id,
                            "enabled": enabled,
                            "name": name,
                            "state": if *enabled { "enabled" } else { "disabled" },
                        })
                    })
                    .collect::<Vec<_>>();
                let notes_root_configured = settings_snapshot
                    .as_ref()
                    .and_then(|s| s.notes_root.as_ref())
                    .is_some();
                let projects_roots = settings_snapshot
                    .as_ref()
                    .map(|s| s.projects_roots.len())
                    .unwrap_or(0);
                let mut remediation = Vec::new();
                if !notes_root_configured {
                    remediation.push("Notes: luma config set --notes-root ~/Notes".to_string());
                }
                if projects_roots == 0 {
                    remediation.push("Projects: luma config set --projects-root ~/dev".to_string());
                }
                for (id, reason) in &self.skipped_modules {
                    remediation.push(format!(
                        "Repair store for {id} ({reason}), then restart Luma"
                    ));
                }
                remediation.push("Grant Accessibility if paste/snippets paste fails".into());
                remediation
                    .push("Notes excludes: luma config set --notes-exclude 'private/*'".into());
                let skipped = self
                    .skipped_modules
                    .iter()
                    .map(|(id, reason)| serde_json::json!({ "id": id, "reason": reason }))
                    .collect::<Vec<_>>();
                let diagnostic = serde_json::json!({
                    "doctor": true,
                    "modules": modules,
                    "skipped_modules": skipped,
                    "settings": {
                        "configured": settings_snapshot.is_some(),
                        "settings_version": settings_snapshot.as_ref().map(|s| s.settings_version),
                        "notes_root_configured": notes_root_configured,
                        "notes_root": settings_snapshot.as_ref().and_then(|s| s.notes_root.clone()),
                        "projects_roots": settings_snapshot.as_ref().map(|s| s.projects_roots.clone()).unwrap_or_default(),
                        "notes_exclude_patterns": settings_snapshot.as_ref().map(|s| s.notes_exclude_patterns.clone()).unwrap_or_default(),
                        "clipboard_retention_days": settings_snapshot.as_ref().map(|s| s.clipboard_retention_days),
                    },
                    "config_commands": {
                        "notes_root": "luma config set --notes-root ~/Notes",
                        "projects_roots": "luma config set --projects-root ~/dev",
                        "notes_exclude": "luma config set --notes-exclude 'private/*'",
                    },
                    "stores": {
                        "settings": if settings_snapshot.is_some() { "ok" } else { "missing" },
                        "diagnostics": if self.diagnostics.is_some() { "ok" } else { "missing" },
                    },
                    "remediation": remediation,
                });
                let _ = self.emit(Event::DiagnosticRaised { diagnostic }).await;
            }
            Command::ShutdownSession => {
                {
                    let _lifecycle = self.search_lifecycle.lock().await;
                    self.cancel_all_searches_locked().await;
                }
                let (modules, op_handles) = {
                    let mut g = self.inner.lock().await;
                    g.session_cancel.cancel();
                    let mut op_handles = Vec::new();
                    for op in g.operations.values_mut() {
                        op.cancel.cancel();
                        if let Some(handle) = op.handle.take() {
                            op_handles.push(handle);
                        }
                    }
                    g.operations.clear();
                    (g.registry.all_modules(), op_handles)
                };
                for handle in op_handles {
                    let _ = handle.await;
                }
                for m in modules {
                    m.teardown().await;
                }
            }
            // Runtime-only enable flip (no settings.toml write). Settings UI uses UpdateSettings.
            Command::SetModuleEnabled { module_id, enabled } => {
                let _ = self.apply_module_enabled(&module_id, enabled).await;
            }
            Command::ExecuteAction {
                operation_id,
                result_id,
                action_id,
                confirmation,
            } => {
                let (item, module) = {
                    let g = self.inner.lock().await;
                    let item = g.results_by_id.get(&result_id).cloned();
                    let module = item
                        .as_ref()
                        .and_then(|i| g.registry.get(i.module_id.as_str()));
                    (item, module)
                };
                let cancel = {
                    let mut g = self.inner.lock().await;
                    let cancel = g.session_cancel.child_token();
                    let module_id = item
                        .as_ref()
                        .map(|i| i.module_id.as_str().to_string())
                        .unwrap_or_default();
                    g.operations.insert(
                        operation_id.clone(),
                        OperationTask {
                            cancel: cancel.clone(),
                            module_id,
                            handle: None,
                        },
                    );
                    cancel
                };
                let engine = self.clone_inner();
                let op_id = operation_id.clone();
                let handle = tokio::spawn(async move {
                    let (tx, broadcast_tx) = {
                        let g = engine.lock().await;
                        (g.event_tx.clone(), g.event_broadcast_tx.clone())
                    };
                    let started = Event::ActionStarted {
                        operation_id: op_id.clone(),
                    };
                    let _ = broadcast_tx.send(started.clone());
                    let _ = tx.send(started).await;

                    let outcome = match (item, module) {
                        (Some(result), Some(module)) => {
                            let actions = module.actions(&result).await;
                            if let Some(action) =
                                actions.into_iter().find(|a| a.id.as_str() == action_id)
                            {
                                let needs_confirm = action.confirmation
                                    || !matches!(action.risk, luma_domain::ActionRisk::Safe);
                                if needs_confirm && !confirmation {
                                    luma_protocol::ActionOutcomeDto::failed(
                                        luma_domain::FailureKind::SecurityDenied {
                                            reason: "confirmation required".into(),
                                        },
                                    )
                                } else {
                                    let out = module
                                        .perform(
                                            crate::module::ActionRequest {
                                                result,
                                                action,
                                                confirmation,
                                            },
                                            cancel,
                                        )
                                        .await;
                                    match out {
                                        crate::module::ActionOutcome::Success { message } => {
                                            luma_protocol::ActionOutcomeDto::Success { message }
                                        }
                                        crate::module::ActionOutcome::Cancelled => {
                                            luma_protocol::ActionOutcomeDto::Cancelled
                                        }
                                        crate::module::ActionOutcome::Failed { kind } => {
                                            luma_protocol::ActionOutcomeDto::failed(kind)
                                        }
                                    }
                                }
                            } else {
                                luma_protocol::ActionOutcomeDto::failed(
                                    luma_domain::FailureKind::NotFound {
                                        entity: format!("action:{action_id}"),
                                    },
                                )
                            }
                        }
                        _ => luma_protocol::ActionOutcomeDto::failed(
                            luma_domain::FailureKind::NotFound {
                                entity: format!("result:{result_id}"),
                            },
                        ),
                    };
                    let (tx, broadcast_tx) = {
                        let g = engine.lock().await;
                        (g.event_tx.clone(), g.event_broadcast_tx.clone())
                    };
                    let finished = Event::ActionFinished {
                        operation_id: op_id.clone(),
                        outcome,
                    };
                    let _ = broadcast_tx.send(finished.clone());
                    let _ = tx.send(finished).await;
                    engine.lock().await.operations.remove(&op_id);
                });
                if let Some(op) = self.inner.lock().await.operations.get_mut(&operation_id) {
                    op.handle = Some(handle);
                }
            }
            Command::ListActions { result_id } => {
                let (item, module) = {
                    let g = self.inner.lock().await;
                    let item = g.results_by_id.get(&result_id).cloned();
                    let module = item
                        .as_ref()
                        .and_then(|i| g.registry.get(i.module_id.as_str()));
                    (item, module)
                };
                let actions = match (item, module) {
                    (Some(result), Some(module)) => {
                        let descriptors = module.actions(&result).await;
                        descriptors
                            .into_iter()
                            .map(|a| luma_protocol::ActionDescriptorDto::from(&a))
                            .collect::<Vec<_>>()
                    }
                    _ => Vec::new(),
                };
                let _ = self
                    .emit(Event::ActionsAvailable { result_id, actions })
                    .await;
            }
            Command::LoadPreview {
                result_id,
                preview_id,
            } => {
                let (item, module) = {
                    let g = self.inner.lock().await;
                    let item = g.results_by_id.get(&result_id).cloned();
                    let module = item
                        .as_ref()
                        .and_then(|i| g.registry.get(i.module_id.as_str()));
                    (item, module)
                };
                let body = match (item, module) {
                    (Some(result), Some(module)) => module
                        .preview(&result)
                        .await
                        .unwrap_or_else(|| result.title.clone()),
                    (Some(result), None) => result
                        .subtitle
                        .clone()
                        .unwrap_or_else(|| result.title.clone()),
                    _ => String::new(),
                };
                let _ = self
                    .emit(Event::PreviewLoaded {
                        result_id,
                        preview_id,
                        body,
                    })
                    .await;
            }
            Command::LoadHub => {
                let modules = {
                    let g = self.inner.lock().await;
                    g.registry.enabled_modules()
                };
                let mut pins = Vec::new();
                for module in modules {
                    let id = module.manifest().id.as_str().to_string();
                    for (pin_id, title, query) in module.hub_pins().await {
                        pins.push(luma_protocol::HubPinDto {
                            id: pin_id,
                            title,
                            module_id: id.clone(),
                            query,
                        });
                    }
                }
                let _ = self.emit(Event::HubLoaded { pins }).await;
            }
            Command::GetSettings => {
                let (rows, version) = {
                    let g = self.inner.lock().await;
                    let rows = g.registry.list();
                    let version = self
                        .settings
                        .as_ref()
                        .and_then(|repo| repo.load_or_default().ok())
                        .map(|s| s.settings_version)
                        .unwrap_or(0);
                    (rows, version)
                };
                let settings = serde_json::json!({
                    "source": if self.settings.is_some() { "config_store" } else { "engine_registry" },
                    "modules": rows.iter().map(|(id, enabled, name)| {
                        serde_json::json!({"id": id, "enabled": enabled, "name": name})
                    }).collect::<Vec<_>>(),
                });
                let _ = self
                    .emit(Event::SettingsChanged { version, settings })
                    .await;
            }
            Command::UpdateSettings {
                patch,
                expected_version,
            } => {
                let Some(settings_repo) = &self.settings else {
                    let _ = self.emit(Event::DiagnosticRaised {
                        diagnostic: serde_json::json!({
                            "settings_update": "failed",
                            "message": "no SettingsRepository configured; refusing non-persistent update"
                        }),
                    }).await;
                    return;
                };
                let current = match settings_repo.load_or_default() {
                    Ok(value) => value,
                    Err(err) => {
                        let _ = self.emit(Event::DiagnosticRaised {
                            diagnostic: serde_json::json!({"settings_update": "failed", "message": err.to_string()}),
                        }).await;
                        return;
                    }
                };
                let mut next = current.clone();
                if let Some(obj) = patch.get("enabled_modules").and_then(|v| v.as_object()) {
                    for (id, value) in obj {
                        if let Some(enabled) = value.as_bool() {
                            next.enabled_modules.insert(id.clone(), enabled);
                        }
                    }
                }
                if let Some(root) = patch.get("notes_root") {
                    if root.is_null() {
                        next.notes_root = None;
                    } else if let Some(s) = root.as_str() {
                        next.notes_root = if s.is_empty() {
                            None
                        } else {
                            Some(s.to_string())
                        };
                    }
                }
                if let Some(roots) = patch.get("projects_roots").and_then(|v| v.as_array()) {
                    next.projects_roots = roots
                        .iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect();
                }
                if let Some(patterns) = patch
                    .get("notes_exclude_patterns")
                    .and_then(|v| v.as_array())
                {
                    next.notes_exclude_patterns = patterns
                        .iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect();
                }
                let roots_changed = next.notes_root != current.notes_root
                    || next.projects_roots != current.projects_roots
                    || next.notes_exclude_patterns != current.notes_exclude_patterns;
                let saved = match settings_repo.update_cas(expected_version, next) {
                    Ok(value) => value,
                    Err(err) => {
                        let _ = self
                            .emit(Event::DiagnosticRaised {
                                diagnostic: serde_json::json!({
                                    "settings_update": "failed",
                                    "expected_version": expected_version,
                                    "message": err.to_string()
                                }),
                            })
                            .await;
                        return;
                    }
                };
                let changes: Vec<(String, bool)> = {
                    let g = self.inner.lock().await;
                    saved
                        .enabled_modules
                        .iter()
                        .filter(|(id, enabled)| g.registry.is_enabled(id) != **enabled)
                        .map(|(id, enabled)| (id.clone(), *enabled))
                        .collect()
                };
                for (id, enabled) in changes {
                    let _ = self.apply_module_enabled(&id, enabled).await;
                }
                if roots_changed {
                    let modules = {
                        let g = self.inner.lock().await;
                        g.registry.all_modules()
                    };
                    for module in modules {
                        module.apply_settings(&saved).await;
                    }
                }
                let rows = {
                    let g = self.inner.lock().await;
                    g.registry.list()
                };
                let _ = self
                    .emit(Event::SettingsChanged {
                        version: saved.settings_version,
                        settings: serde_json::json!({
                            "source": "config_store",
                            "modules": rows.iter().map(|(id, enabled, name)| {
                                serde_json::json!({"id": id, "enabled": enabled, "name": name})
                            }).collect::<Vec<_>>(),
                            "notes_root": saved.notes_root,
                            "projects_roots": saved.projects_roots,
                            "notes_exclude_patterns": saved.notes_exclude_patterns,
                        }),
                    })
                    .await;
            }
            Command::OpenModule { module_id } => {
                let state = {
                    let g = self.inner.lock().await;
                    if g.registry.get(&module_id).is_some() {
                        if g.registry.is_enabled(&module_id) {
                            "open".to_string()
                        } else {
                            "disabled".to_string()
                        }
                    } else {
                        "missing".to_string()
                    }
                };
                let _ = self
                    .emit(Event::ModuleStateChanged { module_id, state })
                    .await;
            }
            Command::CancelOperation { operation_id } => {
                let handle = {
                    let mut g = self.inner.lock().await;
                    match g.operations.get_mut(&operation_id) {
                        Some(op) => {
                            op.cancel.cancel();
                            op.handle.take()
                        }
                        None => {
                            drop(g);
                            let _ = self
                                .emit(Event::ActionFinished {
                                    operation_id,
                                    outcome: luma_protocol::ActionOutcomeDto::failed(
                                        luma_domain::FailureKind::NotFound {
                                            entity: "operation".into(),
                                        },
                                    ),
                                })
                                .await;
                            return;
                        }
                    }
                };
                // Await terminal state so cancel is real for the caller.
                if let Some(handle) = handle {
                    let _ = handle.await;
                }
            }
            Command::ExportDiagnostics => {
                let (rows, settings_version) = {
                    let g = self.inner.lock().await;
                    (
                        g.registry.list(),
                        self.settings.as_ref().and_then(|c| {
                            c.load_or_default()
                                .ok()
                                .map(|settings| settings.settings_version)
                        }),
                    )
                };
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or_default();
                let diagnostic = serde_json::json!({
                    "export": true, "redacted": true, "created_unix_ms": now,
                    "settings_version": settings_version,
                    "modules": rows.iter().map(|(id, enabled, name)| serde_json::json!({
                        "id": id, "enabled": enabled, "name": name
                    })).collect::<Vec<_>>(),
                });
                let diagnostic = match self.diagnostics.as_ref().map(|sink| {
                    let body = serde_json::to_vec_pretty(&diagnostic).unwrap_or_default();
                    sink.write_export(&format!("diagnostics-{now}.json"), &body)
                        .map(|path| (path, diagnostic.clone()))
                }) {
                    Some(Ok((path, mut diagnostic))) => {
                        diagnostic["path"] = path.display().to_string().into();
                        diagnostic
                    }
                    Some(Err(err)) => serde_json::json!({
                        "export": false, "redacted": true, "message": err.to_string()
                    }),
                    None => serde_json::json!({
                        "export": false,
                        "redacted": true,
                        "message": "no DiagnosticsSink configured"
                    }),
                };
                let _ = self.emit(Event::DiagnosticRaised { diagnostic }).await;
            }
        }
    }

    /// Collect events until SearchFinished/Cancelled/Fatal or timeout-ish bound.
    pub async fn query_collect(
        &self,
        query: &str,
        request_id: &str,
    ) -> Result<Vec<SearchItemDto>, String> {
        let mut rx = {
            let mut slot = self.event_rx_slot.lock().await;
            slot.take()
                .ok_or_else(|| "event receiver already taken".to_string())?
        };
        // Re-subscribe is not supported once taken; for CLI we create a dedicated engine per call.
        // Put receiver back pattern: use a fresh channel by reconstructing — simpler approach below.
        // Restore for subsequent use by creating fan-in is Phase 9. For CLI helpers, see `run_query`.
        let _ = &mut rx;
        let _ = (query, request_id);
        Err("use Engine::run_query for one-shot CLI".into())
    }
}

/// One-shot helper for non-interactive CLI: own engine lifecycle for a single query.
pub async fn run_query(
    registry: ModuleRegistry,
    query: &str,
) -> Result<(Vec<SearchItemDto>, Vec<Event>), String> {
    let engine = Engine::new(registry);
    let mut rx = engine
        .take_event_receiver()
        .await
        .expect("fresh engine has receiver");

    engine.start_session().await;
    let request_id = "cli-1".to_string();
    let search = engine.handle_command(Command::Search {
        request_id: request_id.clone(),
        query: query.to_string(),
    });

    let collect = async {
        let mut events = Vec::new();
        let mut items: HashMap<String, SearchItemDto> = HashMap::new();
        while let Some(ev) = rx.recv().await {
            match &ev {
                Event::ResultsChunk { upserts, .. } => {
                    for u in upserts {
                        items.insert(u.id.clone(), u.clone());
                    }
                }
                Event::SearchFinished { .. }
                | Event::SearchCancelled { .. }
                | Event::Fatal { .. } => {
                    events.push(ev);
                    break;
                }
                _ => events.push(ev),
            }
        }
        let mut out: Vec<_> = items.into_values().collect();
        out.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        (out, events)
    };

    let ((), (items, events)) = tokio::join!(search, collect);
    let _ = engine.handle_command(Command::ShutdownSession).await;
    Ok((items, events))
}

/// One-shot CLI action helper. Searches and executes within one engine session so result ids remain valid.
pub async fn run_action(
    registry: ModuleRegistry,
    query: &str,
    result_id: Option<&str>,
    action_id: &str,
    confirmation: bool,
) -> Result<(SearchItemDto, luma_protocol::ActionOutcomeDto), String> {
    let engine = Engine::new(registry);
    let mut rx = engine
        .take_event_receiver()
        .await
        .expect("fresh engine has receiver");
    engine.start_session().await;
    let request_id = "cli-action-search".to_string();
    let search = engine.handle_command(Command::Search {
        request_id,
        query: query.to_string(),
    });
    let collect = async {
        let mut items = HashMap::new();
        while let Some(event) = rx.recv().await {
            match event {
                Event::ResultsChunk { upserts, .. } => {
                    for item in upserts {
                        items.insert(item.id.clone(), item);
                    }
                }
                Event::SearchFinished { .. } | Event::SearchCancelled { .. } => break,
                _ => {}
            }
        }
        items
    };
    let ((), items) = tokio::join!(search, collect);
    let selected = match result_id {
        Some(id) => items
            .get(id)
            .cloned()
            .ok_or_else(|| format!("result not found: {id}"))?,
        None => {
            let mut values: Vec<_> = items.into_values().collect();
            values.sort_by(|a, b| b.score.total_cmp(&a.score));
            values
                .into_iter()
                .next()
                .ok_or_else(|| "query returned no results".to_string())?
        }
    };
    engine
        .handle_command(Command::ExecuteAction {
            operation_id: "cli-action-1".into(),
            result_id: selected.id.clone(),
            action_id: action_id.into(),
            confirmation,
        })
        .await;
    let outcome = loop {
        match rx.recv().await {
            Some(Event::ActionFinished { outcome, .. }) => break outcome,
            Some(_) => {}
            None => return Err("engine event channel closed".into()),
        }
    };
    engine.handle_command(Command::ShutdownSession).await;
    Ok((selected, outcome))
}

pub async fn run_doctor(registry: ModuleRegistry) -> Result<serde_json::Value, String> {
    let engine = Engine::new(registry);
    let mut rx = engine
        .take_event_receiver()
        .await
        .expect("fresh engine has receiver");
    engine.start_session().await;
    let handle = engine.handle_command(Command::RunDoctor);
    let wait = async {
        while let Some(ev) = rx.recv().await {
            if let Event::DiagnosticRaised { diagnostic } = ev {
                return diagnostic;
            }
        }
        serde_json::json!({"error": "no diagnostic"})
    };
    let ((), diagnostic) = tokio::join!(handle, wait);
    let _ = engine.handle_command(Command::ShutdownSession).await;
    Ok(diagnostic)
}

pub async fn list_modules_json(registry: &ModuleRegistry) -> serde_json::Value {
    let rows = registry.list();
    serde_json::json!({
        "modules": rows.iter().map(|(id, enabled, name)| {
            serde_json::json!({ "id": id, "enabled": enabled, "display_name": name })
        }).collect::<Vec<_>>()
    })
}

#[async_trait]
impl EnginePort for Engine {
    async fn submit(&self, command: Command) -> Result<(), String> {
        self.handle_command(command).await;
        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.event_broadcast_tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::{
        ActionOutcome, ActionRequest, LumaModule, ModuleManifest, SearchMode, SearchSink,
        WarmupContext,
    };
    use async_trait::async_trait;
    use luma_domain::{ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, SearchItem};
    use tokio_util::sync::CancellationToken;

    struct FakeModule {
        manifest: ModuleManifest,
        wait_for_cancel: bool,
    }

    #[async_trait]
    impl LumaModule for FakeModule {
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
            let item = SearchItemDto {
                id: if self.wait_for_cancel {
                    "wait-1".into()
                } else {
                    "fake-1".into()
                },
                module_id: self.manifest.id.as_str().to_string(),
                title: format!("Fake: {}", query.normalized),
                subtitle: None,
                kind: "fake".into(),
                score: 42.0,
                primary_action_id: "open".into(),
                primary_action_label: "Open".into(),
                ..Default::default()
            };
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: "ignored".into(),
                    sequence: 1,
                    upserts: vec![item],
                    removed_ids: vec![],
                })
                .await;
        }

        async fn actions(&self, _result: &SearchItem) -> Vec<ActionDescriptor> {
            vec![ActionDescriptor {
                id: ActionId::new("open"),
                label: "Open".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }]
        }

        async fn perform(
            &self,
            _action: ActionRequest,
            cancel: CancellationToken,
        ) -> ActionOutcome {
            if self.wait_for_cancel {
                cancel.cancelled().await;
                return ActionOutcome::Cancelled;
            }
            if cancel.is_cancelled() {
                return ActionOutcome::Cancelled;
            }
            ActionOutcome::Success {
                message: Some("ok".into()),
            }
        }

        async fn teardown(&self) {}
    }

    struct StickySearchModule {
        manifest: ModuleManifest,
        ran_after_sleep: Arc<std::sync::atomic::AtomicBool>,
    }

    #[async_trait]
    impl LumaModule for StickySearchModule {
        fn manifest(&self) -> &ModuleManifest {
            &self.manifest
        }

        async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
            ModuleState::Ready
        }

        async fn search(&self, _query: Query, sink: SearchSink, _cancel: CancellationToken) {
            // Deliberately ignore cancellation; only JoinSet abort should stop us.
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            self.ran_after_sleep
                .store(true, std::sync::atomic::Ordering::SeqCst);
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![SearchItemDto {
                        id: "sticky".into(),
                        module_id: "luma.sticky".into(),
                        title: "late".into(),
                        subtitle: None,
                        kind: "sticky".into(),
                        score: 1.0,
                        primary_action_id: "noop".into(),
                        primary_action_label: "Noop".into(),
                        ..Default::default()
                    }],
                    removed_ids: vec![],
                })
                .await;
        }

        async fn actions(&self, _result: &SearchItem) -> Vec<ActionDescriptor> {
            Vec::new()
        }

        async fn perform(
            &self,
            _action: ActionRequest,
            _cancel: CancellationToken,
        ) -> ActionOutcome {
            ActionOutcome::Success { message: None }
        }

        async fn teardown(&self) {}
    }

    fn fake_registry() -> ModuleRegistry {
        let mut reg = ModuleRegistry::new();
        reg.register(Arc::new(FakeModule {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.fake"),
                display_name: "Fake".into(),
                triggers: vec!["fake".into()],
                default_enabled: true,
                search_mode: SearchMode::GlobalContributing,
                required_capabilities: vec![],
                workbench: Default::default(),
            },
            wait_for_cancel: false,
        }))
        .expect("register fake");
        reg
    }

    #[tokio::test]
    async fn search_cancel_aborts_noncooperative_module_task() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let ran = Arc::new(AtomicBool::new(false));
        let mut registry = ModuleRegistry::new();
        registry
            .register(Arc::new(StickySearchModule {
                manifest: ModuleManifest {
                    id: ModuleId::new("luma.sticky"),
                    display_name: "Sticky".into(),
                    triggers: vec!["sticky".into()],
                    default_enabled: true,
                    search_mode: SearchMode::GlobalContributing,
                    required_capabilities: vec![],
                    workbench: Default::default(),
                },
                ran_after_sleep: ran.clone(),
            }))
            .unwrap();
        let engine = Arc::new(Engine::new(registry));
        let mut events = engine.take_event_receiver().await.unwrap();
        engine.start_session().await;
        engine
            .handle_command(Command::Search {
                request_id: "sticky-1".into(),
                query: "hello".into(),
            })
            .await;
        while !matches!(events.recv().await, Some(Event::SearchStarted { .. })) {}
        engine
            .handle_command(Command::CancelSearch {
                request_id: "sticky-1".into(),
            })
            .await;
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        assert!(
            !ran.load(Ordering::SeqCst),
            "aborted search task must not resume after cancel"
        );
    }

    #[tokio::test]
    async fn cancel_before_registration_is_honored() {
        let engine = Arc::new(Engine::new(fake_registry()));
        let mut events = engine.take_event_receiver().await.unwrap();
        engine.start_session().await;
        while !matches!(events.recv().await, Some(Event::SessionReady { .. })) {}
        // Cancel arrives before Search for the same request id.
        engine
            .handle_command(Command::CancelSearch {
                request_id: "early".into(),
            })
            .await;
        assert!(matches!(
            events.recv().await,
            Some(Event::SearchCancelled { request_id }) if request_id == "early"
        ));
        engine
            .handle_command(Command::Search {
                request_id: "early".into(),
                query: "hello".into(),
            })
            .await;
        // Must not start a live search for a pre-cancelled request.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        while let Ok(ev) = events.try_recv() {
            assert!(
                !matches!(
                    ev,
                    Event::SearchStarted { .. } | Event::SearchFinished { .. }
                ),
                "pre-cancelled search must not start: {ev:?}"
            );
        }
    }

    #[tokio::test]
    async fn cancel_new_search_while_previous_is_tearing_down() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let ran = Arc::new(AtomicBool::new(false));
        let mut registry = ModuleRegistry::new();
        registry
            .register(Arc::new(StickySearchModule {
                manifest: ModuleManifest {
                    id: ModuleId::new("luma.sticky"),
                    display_name: "Sticky".into(),
                    triggers: vec!["sticky".into()],
                    default_enabled: true,
                    search_mode: SearchMode::GlobalContributing,
                    required_capabilities: vec![],
                    workbench: Default::default(),
                },
                ran_after_sleep: ran.clone(),
            }))
            .unwrap();
        let engine = Arc::new(Engine::new(registry));
        let mut events = engine.take_event_receiver().await.unwrap();
        engine.start_session().await;

        engine
            .handle_command(Command::Search {
                request_id: "old".into(),
                query: "hello".into(),
            })
            .await;
        while !matches!(events.recv().await, Some(Event::SearchStarted { .. })) {}

        let engine_b = engine.clone();
        let search_new = tokio::spawn(async move {
            engine_b
                .handle_command(Command::Search {
                    request_id: "new".into(),
                    query: "hello".into(),
                })
                .await;
        });
        // Let the new search acquire lifecycle and begin cancelling the old one.
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        engine
            .handle_command(Command::CancelSearch {
                request_id: "new".into(),
            })
            .await;

        let mut saw_new_cancelled = false;
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
        while tokio::time::Instant::now() < deadline {
            tokio::select! {
                ev = events.recv() => {
                    match ev {
                        Some(Event::SearchCancelled { request_id }) if request_id == "new" => {
                            saw_new_cancelled = true;
                            break;
                        }
                        Some(Event::SearchFinished { request_id, .. }) if request_id == "new" => {
                            panic!("new search must not finish after cancel");
                        }
                        _ => {}
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {}
            }
        }
        assert!(
            saw_new_cancelled,
            "expected SearchCancelled for new request"
        );
        search_new.await.unwrap();
        assert!(
            !ran.load(Ordering::SeqCst),
            "sticky work from cancelled searches must not complete"
        );
    }

    #[tokio::test]
    async fn query_returns_fake_hit() {
        let (items, _events) = run_query(fake_registry(), "hello").await.unwrap();
        assert_eq!(items.len(), 1);
        assert!(items[0].title.contains("hello"));
    }

    #[tokio::test]
    async fn run_action_executes_fake_result() {
        let (result, outcome) = run_action(fake_registry(), "hello", None, "open", false)
            .await
            .unwrap();
        assert_eq!(result.id, "fake-1");
        assert!(matches!(
            outcome,
            luma_protocol::ActionOutcomeDto::Success { .. }
        ));
    }

    #[tokio::test]
    async fn doctor_lists_modules() {
        let diag = run_doctor(fake_registry()).await.unwrap();
        assert_eq!(diag["doctor"], true);
        assert!(diag["modules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["id"] == "luma.fake"));
    }

    #[tokio::test]
    async fn permission_failure_kind_not_empty_success() {
        let kind = FailureKind::PermissionRequired {
            capability: "ax".into(),
            guidance: "enable".into(),
        };
        assert!(kind.is_error());
        let outcome = ActionOutcome::Failed { kind };
        assert!(matches!(outcome, ActionOutcome::Failed { .. }));
    }

    #[tokio::test]
    async fn subscribe_receives_session_ready() {
        use crate::port::EnginePort;

        let engine = Engine::new(fake_registry());
        let mut events = engine.subscribe();
        engine.start_session().await;
        assert!(matches!(
            events.recv().await,
            Ok(Event::ModuleStateChanged { .. })
        ));
        assert!(matches!(
            events.recv().await,
            Ok(Event::SessionReady { .. })
        ));
    }

    #[tokio::test]
    async fn cancel_operation_cancels_in_flight_perform() {
        let mut registry = fake_registry();
        registry
            .register(Arc::new(FakeModule {
                manifest: ModuleManifest {
                    id: ModuleId::new("luma.wait"),
                    display_name: "Wait".into(),
                    triggers: vec!["wait".into()],
                    default_enabled: true,
                    search_mode: SearchMode::GlobalContributing,
                    required_capabilities: vec![],
                    workbench: Default::default(),
                },
                wait_for_cancel: true,
            }))
            .unwrap();
        let engine = Arc::new(Engine::new(registry));
        let mut events = engine.take_event_receiver().await.unwrap();
        engine.start_session().await;
        engine
            .handle_command(Command::Search {
                request_id: "r1".into(),
                query: "wait hello".into(),
            })
            .await;
        while !matches!(events.recv().await, Some(Event::SearchFinished { .. })) {}
        let execute = {
            let engine = engine.clone();
            tokio::spawn(async move {
                engine
                    .handle_command(Command::ExecuteAction {
                        operation_id: "op1".into(),
                        result_id: "wait-1".into(),
                        action_id: "open".into(),
                        confirmation: false,
                    })
                    .await;
            })
        };
        while !matches!(events.recv().await, Some(Event::ActionStarted { .. })) {}
        engine
            .handle_command(Command::CancelOperation {
                operation_id: "op1".into(),
            })
            .await;
        let outcome = loop {
            if let Some(Event::ActionFinished { outcome, .. }) = events.recv().await {
                break outcome;
            }
        };
        assert!(matches!(
            outcome,
            luma_protocol::ActionOutcomeDto::Cancelled
        ));
        execute.await.unwrap();
    }

    #[tokio::test]
    async fn disable_module_cancels_in_flight_perform() {
        let mut registry = fake_registry();
        registry
            .register(Arc::new(FakeModule {
                manifest: ModuleManifest {
                    id: ModuleId::new("luma.wait"),
                    display_name: "Wait".into(),
                    triggers: vec!["wait".into()],
                    default_enabled: true,
                    search_mode: SearchMode::GlobalContributing,
                    required_capabilities: vec![],
                    workbench: Default::default(),
                },
                wait_for_cancel: true,
            }))
            .unwrap();
        let engine = Arc::new(Engine::new(registry));
        let mut events = engine.take_event_receiver().await.unwrap();
        engine.start_session().await;
        engine
            .handle_command(Command::Search {
                request_id: "r1".into(),
                query: "wait hello".into(),
            })
            .await;
        while !matches!(events.recv().await, Some(Event::SearchFinished { .. })) {}
        let execute = {
            let engine = engine.clone();
            tokio::spawn(async move {
                engine
                    .handle_command(Command::ExecuteAction {
                        operation_id: "op-disable".into(),
                        result_id: "wait-1".into(),
                        action_id: "open".into(),
                        confirmation: false,
                    })
                    .await;
            })
        };
        while !matches!(events.recv().await, Some(Event::ActionStarted { .. })) {}
        engine
            .handle_command(Command::SetModuleEnabled {
                module_id: "luma.wait".into(),
                enabled: false,
            })
            .await;
        let outcome = loop {
            if let Some(Event::ActionFinished { outcome, .. }) = events.recv().await {
                break outcome;
            }
        };
        assert!(matches!(
            outcome,
            luma_protocol::ActionOutcomeDto::Cancelled
        ));
        execute.await.unwrap();
    }

    #[tokio::test]
    async fn start_session_skips_warmup_for_disabled_modules() {
        let mut registry = fake_registry();
        let _ = registry.set_enabled("luma.fake", false);
        let engine = Engine::new(registry);
        let mut events = engine.subscribe();
        engine.start_session().await;
        let first = events.recv().await.unwrap();
        assert!(matches!(
            first,
            Event::ModuleStateChanged { ref module_id, ref state }
                if module_id == "luma.fake" && state == "disabled"
        ));
        assert!(matches!(
            events.recv().await,
            Ok(Event::SessionReady { .. })
        ));
    }

    #[tokio::test]
    async fn update_settings_persists_with_config_store() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(luma_storage::ConfigStore::with_path(
            dir.path().join("settings.toml"),
        ));
        let settings = Arc::new(crate::TomlSettingsRepository::new(store.clone()));
        let engine = Engine::with_settings(fake_registry(), Some(settings));
        let mut events = engine.take_event_receiver().await.unwrap();
        engine
            .handle_command(Command::UpdateSettings {
                patch: serde_json::json!({"enabled_modules": {"luma.fake": false}}),
                expected_version: 1,
            })
            .await;
        let event = loop {
            if let Some(Event::SettingsChanged { version, settings }) = events.recv().await {
                break (version, settings);
            }
        };
        assert_eq!(event.0, 2);
        let modules = event.1["modules"].as_array().expect("modules array");
        assert!(
            modules
                .iter()
                .any(|m| m["id"] == "luma.fake" && m["enabled"] == false),
            "{modules:?}"
        );
        assert!(!store.load_or_default().unwrap().enabled_modules["luma.fake"]);
    }

    #[tokio::test]
    async fn removed_ids_evict_results_by_id() {
        struct RemoveModule {
            manifest: ModuleManifest,
        }

        #[async_trait]
        impl LumaModule for RemoveModule {
            fn manifest(&self) -> &ModuleManifest {
                &self.manifest
            }

            async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
                ModuleState::Ready
            }

            async fn search(&self, _query: Query, sink: SearchSink, cancel: CancellationToken) {
                if cancel.is_cancelled() {
                    return;
                }
                let item = SearchItemDto {
                    id: "ephemeral-1".into(),
                    module_id: self.manifest.id.as_str().to_string(),
                    title: "Ephemeral".into(),
                    subtitle: None,
                    kind: "fake".into(),
                    score: 1.0,
                    primary_action_id: "open".into(),
                    primary_action_label: "Open".into(),
                    ..Default::default()
                };
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![item],
                        removed_ids: vec![],
                    })
                    .await;
                if cancel.is_cancelled() {
                    return;
                }
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 2,
                        upserts: vec![],
                        removed_ids: vec!["ephemeral-1".into()],
                    })
                    .await;
            }

            async fn actions(&self, _result: &SearchItem) -> Vec<ActionDescriptor> {
                vec![ActionDescriptor {
                    id: ActionId::new("open"),
                    label: "Open".into(),
                    risk: ActionRisk::Safe,
                    confirmation: false,
                }]
            }

            async fn perform(
                &self,
                _action: ActionRequest,
                _cancel: CancellationToken,
            ) -> ActionOutcome {
                ActionOutcome::Success {
                    message: Some("ok".into()),
                }
            }

            async fn teardown(&self) {}
        }

        let mut registry = ModuleRegistry::new();
        registry
            .register(Arc::new(RemoveModule {
                manifest: ModuleManifest {
                    id: ModuleId::new("luma.remove"),
                    display_name: "Remove".into(),
                    triggers: vec!["rm".into()],
                    default_enabled: true,
                    search_mode: SearchMode::TargetedOnly,
                    required_capabilities: vec![],
                    workbench: Default::default(),
                },
            }))
            .unwrap();
        let engine = Arc::new(Engine::new(registry));
        let mut events = engine.take_event_receiver().await.unwrap();
        engine.start_session().await;
        while !matches!(events.recv().await, Some(Event::SessionReady { .. })) {}
        engine
            .handle_command(Command::Search {
                request_id: "r-rm".into(),
                query: "rm x".into(),
            })
            .await;
        while !matches!(events.recv().await, Some(Event::SearchFinished { .. })) {}

        engine
            .handle_command(Command::ListActions {
                result_id: "ephemeral-1".into(),
            })
            .await;
        let actions = loop {
            if let Some(Event::ActionsAvailable { actions, .. }) = events.recv().await {
                break actions;
            }
        };
        assert!(
            actions.is_empty(),
            "removed id must not resolve actions: {actions:?}"
        );

        engine
            .handle_command(Command::ExecuteAction {
                operation_id: "op-rm".into(),
                result_id: "ephemeral-1".into(),
                action_id: "open".into(),
                confirmation: false,
            })
            .await;
        let outcome = loop {
            if let Some(Event::ActionFinished { outcome, .. }) = events.recv().await {
                break outcome;
            }
        };
        assert!(
            matches!(outcome, luma_protocol::ActionOutcomeDto::Failed { .. }),
            "removed id must not execute: {outcome:?}"
        );
    }

    #[tokio::test]
    async fn export_diagnostics_creates_redacted_file() {
        let dir = tempfile::tempdir().unwrap();
        let sink = Arc::new(crate::FsDiagnosticsSink::new(dir.path()));
        let engine = Engine::with_options(
            fake_registry(),
            EngineOptions {
                settings: None,
                diagnostics: Some(sink),
                skipped_modules: Vec::new(),
            },
        );
        let mut events = engine.take_event_receiver().await.unwrap();
        engine.handle_command(Command::ExportDiagnostics).await;
        let path = loop {
            if let Some(Event::DiagnosticRaised { diagnostic }) = events.recv().await {
                break diagnostic["path"].as_str().unwrap().to_owned();
            }
        };
        let body = std::fs::read_to_string(path).unwrap();
        assert!(body.contains("\"redacted\": true"));
        assert!(!body.contains("clipboard_text"));
    }
}
