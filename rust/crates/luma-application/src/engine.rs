use crate::module::{LumaModule, ModuleState, WarmupContext};
use crate::port::EnginePort;
use crate::registry::ModuleRegistry;
use async_trait::async_trait;
use luma_domain::{Query, QueryScope};
use luma_protocol::{Command, Event, SearchItemDto};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::warn;

struct SearchTask {
    cancel: CancellationToken,
    handle: JoinHandle<()>,
}

struct EngineInner {
    registry: ModuleRegistry,
    event_tx: mpsc::Sender<Event>,
    event_broadcast_tx: broadcast::Sender<Event>,
    session_cancel: CancellationToken,
    searches: HashMap<String, SearchTask>,
    operations: HashMap<String, CancellationToken>,
    results_by_id: HashMap<String, luma_domain::SearchItem>,
}

/// In-process engine: owns modules, searches, and operations.
pub struct Engine {
    inner: Arc<Mutex<EngineInner>>,
    event_rx_slot: Mutex<Option<mpsc::Receiver<Event>>>,
    event_broadcast_tx: broadcast::Sender<Event>,
    config: Option<Arc<luma_storage::ConfigStore>>,
}

impl Engine {
    pub fn new(registry: ModuleRegistry) -> Self {
        Self::with_config(registry, None)
    }

    pub fn with_config(
        registry: ModuleRegistry,
        config: Option<Arc<luma_storage::ConfigStore>>,
    ) -> Self {
        let (event_tx, event_rx) = mpsc::channel(256);
        let (event_broadcast_tx, _) = broadcast::channel(256);
        Self {
            inner: Arc::new(Mutex::new(EngineInner {
                registry,
                event_tx,
                event_broadcast_tx: event_broadcast_tx.clone(),
                session_cancel: CancellationToken::new(),
                searches: HashMap::new(),
                operations: HashMap::new(),
                results_by_id: HashMap::new(),
            })),
            event_rx_slot: Mutex::new(Some(event_rx)),
            event_broadcast_tx,
            config,
        }
    }

    /// Take the unique event receiver (once). Used by TUI and one-shot CLI helpers.
    pub async fn take_event_receiver(&self) -> Option<mpsc::Receiver<Event>> {
        self.event_rx_slot.lock().await.take()
    }

    pub async fn start_session(&self) {
        let modules = {
            let g = self.inner.lock().await;
            g.registry.all_modules()
        };
        for module in modules {
            let cancel = {
                let g = self.inner.lock().await;
                g.session_cancel.child_token()
            };
            let state = module
                .warmup(WarmupContext {
                    cancel: cancel.clone(),
                })
                .await;
            let id = module.manifest().id.as_str().to_string();
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
        let _ = self.emit(Event::SessionReady).await;
    }

    async fn emit(&self, event: Event) -> Result<(), String> {
        let (tx, broadcast_tx) = {
            let g = self.inner.lock().await;
            (g.event_tx.clone(), g.event_broadcast_tx.clone())
        };
        let _ = broadcast_tx.send(event.clone());
        tx.send(event).await.map_err(|e| e.to_string())
    }

    fn cancel_search_locked(inner: &mut EngineInner, request_id: &str) {
        if let Some(task) = inner.searches.remove(request_id) {
            task.cancel.cancel();
            // Do not await join here while holding lock long; abort is enough for Phase 2.
            task.handle.abort();
        }
    }

    async fn handle_search(&self, request_id: String, query_raw: String) {
        {
            let mut g = self.inner.lock().await;
            // New search cancels all previous in-flight searches for simplicity in Phase 2.
            let ids: Vec<String> = g.searches.keys().cloned().collect();
            for id in ids {
                Self::cancel_search_locked(&mut g, &id);
            }
            g.results_by_id.clear();
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

        let query = Query::parse(query_raw, 50);
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
            let _ = self
                .emit(Event::SearchFinished {
                    request_id,
                    total: 0,
                    elapsed_ms: 0,
                })
                .await;
            return;
        }

        let cancel = {
            let g = self.inner.lock().await;
            g.session_cancel.child_token()
        };
        let (chunk_tx, mut chunk_rx) = mpsc::channel::<Event>(64);
        let engine = self.clone_inner();
        let request_for_task = request_id.clone();
        let cancel_for_task = cancel.clone();

        let handles: Vec<_> = modules
            .into_iter()
            .map(|module| {
                let q = query.clone();
                let sink = chunk_tx.clone();
                let token = cancel_for_task.clone();
                tokio::spawn(async move {
                    module.search(q, sink, token).await;
                })
            })
            .collect();
        drop(chunk_tx);

        let collect = tokio::spawn({
            let request_id = request_id.clone();
            let engine = engine.clone();
            async move {
                let mut sequence = 0u64;
                let mut total = 0usize;
                while let Some(ev) = chunk_rx.recv().await {
                    if cancel_for_task.is_cancelled() {
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
                if cancel_for_task.is_cancelled() {
                    let (tx, broadcast_tx) = {
                        let g = engine.lock().await;
                        (g.event_tx.clone(), g.event_broadcast_tx.clone())
                    };
                    let event = Event::SearchCancelled { request_id };
                    let _ = broadcast_tx.send(event.clone());
                    let _ = tx.send(event).await;
                } else {
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

        {
            let mut g = self.inner.lock().await;
            g.searches.insert(
                request_for_task,
                SearchTask {
                    cancel,
                    handle: collect,
                },
            );
        }

        for h in handles {
            if let Err(err) = h.await {
                if !err.is_cancelled() {
                    warn!(?err, "module search task ended with error");
                }
            }
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
                let mut g = self.inner.lock().await;
                Self::cancel_search_locked(&mut g, &request_id);
                drop(g);
                let _ = self.emit(Event::SearchCancelled { request_id }).await;
            }
            Command::RunDoctor => {
                let rows = {
                    let g = self.inner.lock().await;
                    g.registry.list()
                };
                let diagnostic = serde_json::json!({
                    "doctor": true,
                    "modules": rows.iter().map(|(id, enabled, name)| {
                        serde_json::json!({"id": id, "enabled": enabled, "name": name})
                    }).collect::<Vec<_>>(),
                });
                let _ = self.emit(Event::DiagnosticRaised { diagnostic }).await;
            }
            Command::ShutdownSession => {
                let mut g = self.inner.lock().await;
                g.session_cancel.cancel();
                let ids: Vec<String> = g.searches.keys().cloned().collect();
                for id in ids {
                    Self::cancel_search_locked(&mut g, &id);
                }
                for cancel in g.operations.values() {
                    cancel.cancel();
                }
                let modules = g.registry.all_modules();
                drop(g);
                for m in modules {
                    m.teardown().await;
                }
            }
            Command::SetModuleEnabled { module_id, enabled } => {
                let mut g = self.inner.lock().await;
                let ok = g.registry.set_enabled(&module_id, enabled);
                drop(g);
                if ok {
                    let _ = self
                        .emit(Event::ModuleStateChanged {
                            module_id,
                            state: if enabled {
                                "enabled".into()
                            } else {
                                "disabled".into()
                            },
                        })
                        .await;
                }
            }
            Command::ExecuteAction {
                operation_id,
                result_id,
                action_id,
                confirmation,
            } => {
                let cancel = {
                    let mut g = self.inner.lock().await;
                    let cancel = g.session_cancel.child_token();
                    g.operations.insert(operation_id.clone(), cancel.clone());
                    cancel
                };
                let _ = self
                    .emit(Event::ActionStarted {
                        operation_id: operation_id.clone(),
                    })
                    .await;
                let (item, module) = {
                    let g = self.inner.lock().await;
                    let item = g.results_by_id.get(&result_id).cloned();
                    let module = item
                        .as_ref()
                        .and_then(|i| g.registry.get(i.module_id.as_str()));
                    (item, module)
                };
                let outcome = match (item, module) {
                    (Some(result), Some(module)) => {
                        let actions = module.actions(&result).await;
                        if let Some(action) =
                            actions.into_iter().find(|a| a.id.as_str() == action_id)
                        {
                            if action.confirmation && !confirmation {
                                luma_protocol::ActionOutcomeDto::Failed {
                                    message: "confirmation required".into(),
                                }
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
                                        luma_protocol::ActionOutcomeDto::Failed {
                                            message: format!("{kind:?}"),
                                        }
                                    }
                                }
                            }
                        } else {
                            luma_protocol::ActionOutcomeDto::Failed {
                                message: "action not found".into(),
                            }
                        }
                    }
                    _ => luma_protocol::ActionOutcomeDto::Failed {
                        message: "result not found in engine".into(),
                    },
                };
                let _ = self
                    .emit(Event::ActionFinished {
                        operation_id: operation_id.clone(),
                        outcome,
                    })
                    .await;
                self.inner.lock().await.operations.remove(&operation_id);
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
                            .map(|a| {
                                serde_json::json!({
                                    "id": a.id.as_str(),
                                    "label": a.label,
                                    "risk": format!("{:?}", a.risk),
                                    "confirmation": a.confirmation,
                                })
                            })
                            .collect::<Vec<_>>()
                    }
                    _ => Vec::new(),
                };
                let _ = self
                    .emit(Event::ActionsAvailable { result_id, actions })
                    .await;
            }
            Command::GetSettings => {
                let rows = {
                    let g = self.inner.lock().await;
                    g.registry.list()
                };
                let settings = serde_json::json!({
                    "source": "engine_registry",
                    "modules": rows.iter().map(|(id, enabled, name)| {
                        serde_json::json!({"id": id, "enabled": enabled, "name": name})
                    }).collect::<Vec<_>>(),
                });
                let _ = self
                    .emit(Event::SettingsChanged {
                        version: 0,
                        settings,
                    })
                    .await;
            }
            Command::UpdateSettings {
                patch,
                expected_version,
            } => {
                let Some(config) = &self.config else {
                    let _ = self.emit(Event::DiagnosticRaised {
                        diagnostic: serde_json::json!({
                            "settings_update": "failed",
                            "message": "no ConfigStore configured; refusing non-persistent update"
                        }),
                    }).await;
                    return;
                };
                let current = match config.load_or_default() {
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
                let saved = match config.update_cas(expected_version, next) {
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
                {
                    let mut g = self.inner.lock().await;
                    for (id, enabled) in &saved.enabled_modules {
                        let _ = g.registry.set_enabled(id, *enabled);
                    }
                }
                let _ = self
                    .emit(Event::SettingsChanged {
                        version: saved.settings_version,
                        settings: serde_json::json!({
                            "source": "config_store",
                            "modules": saved.enabled_modules,
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
                let cancel = self
                    .inner
                    .lock()
                    .await
                    .operations
                    .get(&operation_id)
                    .cloned();
                match cancel {
                    Some(cancel) => cancel.cancel(),
                    None => {
                        let _ = self
                            .emit(Event::ActionFinished {
                                operation_id,
                                outcome: luma_protocol::ActionOutcomeDto::Failed {
                                    message: "operation not found or already finished".into(),
                                },
                            })
                            .await;
                    }
                }
            }
            Command::ExportDiagnostics => {
                let (rows, settings_version) = {
                    let g = self.inner.lock().await;
                    (
                        g.registry.list(),
                        self.config.as_ref().and_then(|c| {
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
                let diagnostic = match luma_storage::luma_next_logs_dir().and_then(|dir| {
                    std::fs::create_dir_all(&dir).map_err(luma_storage::PathsError::from)?;
                    let path = dir.join(format!("diagnostics-{now}.json"));
                    std::fs::write(
                        &path,
                        serde_json::to_vec_pretty(&diagnostic).unwrap_or_default(),
                    )
                    .map_err(luma_storage::PathsError::from)?;
                    Ok((path, diagnostic))
                }) {
                    Ok((path, mut diagnostic)) => {
                        diagnostic["path"] = path.display().to_string().into();
                        diagnostic
                    }
                    Err(err) => serde_json::json!({
                        "export": false, "redacted": true, "message": err.to_string()
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
            },
            wait_for_cancel: false,
        }));
        reg
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
        assert!(matches!(events.recv().await, Ok(Event::SessionReady)));
    }

    #[tokio::test]
    async fn cancel_operation_cancels_in_flight_perform() {
        let mut registry = fake_registry();
        registry.register(Arc::new(FakeModule {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.wait"),
                display_name: "Wait".into(),
                triggers: vec!["wait".into()],
                default_enabled: true,
                search_mode: SearchMode::GlobalContributing,
                required_capabilities: vec![],
            },
            wait_for_cancel: true,
        }));
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
    async fn update_settings_persists_with_config_store() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(luma_storage::ConfigStore::with_path(
            dir.path().join("settings.toml"),
        ));
        let engine = Engine::with_config(fake_registry(), Some(store.clone()));
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
        assert_eq!(event.1["modules"]["luma.fake"], false);
        assert!(!store.load_or_default().unwrap().enabled_modules["luma.fake"]);
    }

    #[tokio::test]
    async fn export_diagnostics_creates_redacted_file() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("LUMA_NEXT_LOGS_DIR", dir.path());
        let engine = Engine::new(fake_registry());
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
        std::env::remove_var("LUMA_NEXT_LOGS_DIR");
    }
}
