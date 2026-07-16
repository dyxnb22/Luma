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
const OPERATION_CANCEL_BOUND: Duration = Duration::from_millis(750);
/// Soft bound for module search completion; partial results are kept.
#[cfg(test)]
pub(crate) const SEARCH_COMPLETION_BOUND: Duration = Duration::from_millis(300);
#[cfg(not(test))]
pub(crate) const SEARCH_COMPLETION_BOUND: Duration = Duration::from_secs(5);

fn is_meta_prefix(token: &str) -> bool {
    matches!(token, "help")
}

mod actions;
mod search;
mod session;

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

pub(crate) struct EngineInner {
    registry: ModuleRegistry,
    event_broadcast_tx: broadcast::Sender<Event>,
    session_cancel: CancellationToken,
    searches: HashMap<String, SearchTask>,
    /// Cancel arrived before Search registered — Search must abort on start.
    cancel_intents: HashMap<String, ()>,
    /// Search registered under lifecycle but not yet promoted to `searches`.
    pending_searches: HashMap<String, CancellationToken>,
    operations: HashMap<String, OperationTask>,
    results_by_id: HashMap<String, luma_domain::SearchItem>,
    /// Last known warmup/runtime state per module (for honesty / capability gating).
    module_states: HashMap<String, String>,
}

/// Optional infrastructure injected at composition time.
#[derive(Default)]
pub struct EngineOptions {
    pub settings: Option<Arc<dyn crate::ports::SettingsRepository>>,
    pub wordbook: Option<Arc<dyn crate::ports::WordbookRepository>>,
    pub command_recipes: Option<Arc<dyn crate::ports::CommandRecipesRepository>>,
}

/// In-process engine: owns modules, searches, and operations.
pub struct Engine {
    inner: Arc<Mutex<EngineInner>>,
    event_broadcast_tx: broadcast::Sender<Event>,
    settings: Option<Arc<dyn crate::ports::SettingsRepository>>,
    wordbook: Option<Arc<dyn crate::ports::WordbookRepository>>,
    command_recipes: Option<Arc<dyn crate::ports::CommandRecipesRepository>>,
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
                wordbook: None,
                command_recipes: None,
            },
        )
    }

    pub fn with_options(registry: ModuleRegistry, options: EngineOptions) -> Self {
        let (event_broadcast_tx, _) = broadcast::channel(256);
        Self {
            inner: Arc::new(Mutex::new(EngineInner {
                registry,
                event_broadcast_tx: event_broadcast_tx.clone(),
                session_cancel: CancellationToken::new(),
                searches: HashMap::new(),
                cancel_intents: HashMap::new(),
                pending_searches: HashMap::new(),
                operations: HashMap::new(),
                results_by_id: HashMap::new(),
                module_states: HashMap::new(),
            })),
            event_broadcast_tx,
            settings: options.settings,
            wordbook: options.wordbook,
            command_recipes: options.command_recipes,
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
}

impl Engine {
    pub(super) fn clone_inner(&self) -> Arc<Mutex<EngineInner>> {
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
            Command::ShutdownSession => self.handle_shutdown_session().await,
            Command::SetModuleEnabled { module_id, enabled } => {
                let _ = self.apply_module_enabled(&module_id, enabled).await;
            }
            Command::ExecuteAction {
                operation_id,
                result_id,
                action_id,
                confirmation,
            } => {
                self.handle_execute_action(operation_id, result_id, action_id, confirmation)
                    .await;
            }
            Command::ListActions { result_id } => {
                self.handle_list_actions(result_id).await;
            }
            Command::GetSnapshot => {
                let (items, module_states) = {
                    let g = self.inner.lock().await;
                    let items: Vec<SearchItemDto> =
                        g.results_by_id.values().map(SearchItemDto::from).collect();
                    (items, g.module_states.clone())
                };
                let _ = self
                    .emit(Event::SnapshotLoaded {
                        items,
                        module_states,
                    })
                    .await;
            }
            Command::LoadPreview {
                result_id,
                preview_id,
            } => {
                self.handle_load_preview(result_id, preview_id).await;
            }
            Command::LoadHub => {
                let modules = {
                    let g = self.inner.lock().await;
                    g.registry.enabled_modules()
                };
                let mut windows_dto: Option<luma_protocol::HubWindowsDto> = None;
                let mut seeded: Vec<luma_domain::SearchItem> = Vec::new();
                for module in modules {
                    if windows_dto.is_none() {
                        if let Some(slice) = module.hub_windows().await {
                            for row in &slice.windows {
                                seeded.push(luma_domain::SearchItem {
                                    id: luma_domain::ResultId::new(row.id.clone()),
                                    module_id: luma_domain::ModuleId::new("luma.windows"),
                                    title: row.title.clone(),
                                    subtitle: Some(slice.app_name.clone()),
                                    kind: "window".into(),
                                    score: 50.0,
                                    primary_action: luma_domain::ActionDescriptor {
                                        id: luma_domain::ActionId::new("focus"),
                                        label: "Focus".into(),
                                        risk: luma_domain::ActionRisk::Safe,
                                        confirmation: false,
                                    },
                                    secondary_actions: vec![],
                                    ui_intent: None,
                                    action_payload: None,
                                });
                            }
                            windows_dto = Some(luma_protocol::HubWindowsDto {
                                app_name: slice.app_name,
                                windows: slice
                                    .windows
                                    .into_iter()
                                    .map(|w| luma_protocol::HubWindowDto {
                                        id: w.id,
                                        title: w.title,
                                    })
                                    .collect(),
                                more: slice.more,
                                status: slice.status.map(|s| luma_protocol::HubWindowsStatusDto {
                                    kind: s.kind,
                                    title: s.title,
                                    subtitle: s.subtitle,
                                }),
                            });
                        }
                    }
                }
                {
                    let mut g = self.inner.lock().await;
                    for item in seeded {
                        g.results_by_id.insert(item.id.as_str().to_string(), item);
                    }
                }
                let _ = self
                    .emit(Event::HubLoaded {
                        windows: windows_dto,
                    })
                    .await;
            }
            Command::LoadWordbookReview { queue } => {
                self.handle_load_wordbook_review(queue).await;
            }
            Command::GetSettings => {
                let (rows, version, notes_root, projects_roots, imported_projects) = {
                    let g = self.inner.lock().await;
                    let rows = g.registry.list();
                    let snapshot = self
                        .settings
                        .as_ref()
                        .and_then(|repo| repo.load_or_default().ok());
                    let version = snapshot.as_ref().map(|s| s.settings_version).unwrap_or(0);
                    let notes_root = snapshot.as_ref().and_then(|s| s.notes_root.clone());
                    let projects_roots = snapshot
                        .as_ref()
                        .map(|s| s.projects_roots.clone())
                        .unwrap_or_default();
                    let imported_projects = snapshot
                        .as_ref()
                        .map(|s| s.imported_projects.clone())
                        .unwrap_or_default();
                    (rows, version, notes_root, projects_roots, imported_projects)
                };
                let settings = serde_json::json!({
                    "source": if self.settings.is_some() { "config_store" } else { "engine_registry" },
                    "notes_root": notes_root,
                    "projects_roots": projects_roots,
                    "imported_projects": imported_projects,
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
                if let Some(path) = patch.get("import_project").and_then(|v| v.as_str()) {
                    if let Err(err) = next.import_project_path(std::path::Path::new(path)) {
                        let _ = self
                            .emit(Event::DiagnosticRaised {
                                diagnostic: serde_json::json!({
                                    "settings_update": "failed",
                                    "message": err,
                                }),
                            })
                            .await;
                        return;
                    }
                }
                if let Some(name) = patch.get("remove_project").and_then(|v| v.as_str()) {
                    if let Err(err) = next.remove_imported_project(name) {
                        let _ = self
                            .emit(Event::DiagnosticRaised {
                                diagnostic: serde_json::json!({
                                    "settings_update": "failed",
                                    "message": err,
                                }),
                            })
                            .await;
                        return;
                    }
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
                    || next.records_root != current.records_root
                    || next.projects_roots != current.projects_roots
                    || next.imported_projects != current.imported_projects
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
                        g.registry.enabled_modules().into_iter().collect::<Vec<_>>()
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
                            "imported_projects": saved.imported_projects,
                            "notes_exclude_patterns": saved.notes_exclude_patterns,
                        }),
                    })
                    .await;
            }
            Command::CancelOperation { operation_id } => {
                self.handle_cancel_operation(operation_id).await;
            }
            Command::RecordRecipeRun {
                recipe_id,
                result,
                now_unix,
            } => {
                if let Some(repo) = &self.command_recipes {
                    if let Err(err) = repo.record_run(&recipe_id, result, now_unix) {
                        warn!(
                            recipe_id = %recipe_id,
                            error = %err,
                            "failed to record recipe run"
                        );
                    }
                }
            }
            Command::RefreshWordbookReviewStats => {
                self.handle_refresh_wordbook_review_stats().await;
            }
        }
    }

    async fn handle_refresh_wordbook_review_stats(&self) {
        use luma_protocol::WordbookStatsDto;
        let Some(repo) = &self.wordbook else {
            return;
        };
        let stats = repo.stats().unwrap_or_default();
        let _ = self
            .emit(Event::WordbookReviewStatsUpdated {
                stats: WordbookStatsDto {
                    due: stats.due,
                    new_count: stats.new_count,
                    wrong: stats.wrong,
                    goal: stats.goal,
                    reviewed_today: stats.reviewed_today,
                    remaining_goal: stats.remaining_goal,
                },
            })
            .await;
    }

    async fn handle_load_wordbook_review(&self, queue: String) {
        use luma_protocol::{WordReviewWordDto, WordbookStatsDto};
        let Some(repo) = &self.wordbook else {
            let _ = self
                .emit(Event::WordbookReviewLoaded {
                    queue,
                    words: vec![],
                    stats: WordbookStatsDto {
                        due: 0,
                        new_count: 0,
                        wrong: 0,
                        goal: 0,
                        reviewed_today: 0,
                        remaining_goal: 0,
                    },
                })
                .await;
            return;
        };
        let stats = repo.stats().unwrap_or_default();
        let queue_available = match queue.as_str() {
            "new" => stats.new_count.max(0) as usize,
            "wrong" => stats.wrong.max(0) as usize,
            _ => stats.due.max(0) as usize,
        };
        let goal_batch = if stats.remaining_goal > 0 {
            stats.remaining_goal as usize
        } else {
            stats.goal.max(1) as usize
        };
        let limit = goal_batch.min(queue_available.max(1)).clamp(1, 500);
        let words_result = match queue.as_str() {
            "new" => repo.list_new(limit),
            "wrong" => repo.list_wrong(limit),
            _ => repo.list_due(limit),
        };
        let words: Vec<WordReviewWordDto> = match words_result {
            Ok(entries) => {
                let mut words = Vec::with_capacity(entries.len());
                let mut review_items = Vec::with_capacity(entries.len());
                for (index, word) in entries.into_iter().enumerate() {
                    // Review actions use the same `wb:<id>` result contract as
                    // normal Wordbook search. Register the loaded words before
                    // emitting the event so grading does not depend on a prior
                    // search having populated the session cache.
                    review_items.push(luma_domain::SearchItem {
                        id: luma_domain::ResultId::new(format!("wb:{}", word.id)),
                        module_id: luma_domain::ModuleId::new("luma.wordbook"),
                        title: word.term.clone(),
                        subtitle: Some(word.meaning.clone()),
                        kind: "word".into(),
                        score: 90.0 - index as f64 * 0.1,
                        primary_action: luma_domain::ActionDescriptor {
                            id: luma_domain::ActionId::new("known"),
                            label: "Known".into(),
                            risk: luma_domain::ActionRisk::Safe,
                            confirmation: false,
                        },
                        secondary_actions: vec![],
                        ui_intent: None,
                        action_payload: None,
                    });
                    words.push(WordReviewWordDto {
                        id: word.id,
                        term: word.term,
                        phonetic: word.phonetic,
                        meaning: word.meaning,
                        example: word.example,
                    });
                }
                let mut inner = self.inner.lock().await;
                for item in review_items {
                    inner
                        .results_by_id
                        .insert(item.id.as_str().to_string(), item);
                }
                words
            }
            Err(_) => vec![],
        };
        let _ = self
            .emit(Event::WordbookReviewLoaded {
                queue,
                words,
                stats: WordbookStatsDto {
                    due: stats.due,
                    new_count: stats.new_count,
                    wrong: stats.wrong,
                    goal: stats.goal,
                    reviewed_today: stats.reviewed_today,
                    remaining_goal: stats.remaining_goal,
                },
            })
            .await;
    }
}

async fn apply_settings_mutation(
    settings_repo: Option<&Arc<dyn crate::ports::SettingsRepository>>,
    inner: &Arc<Mutex<EngineInner>>,
    patch: serde_json::Value,
) -> Result<String, luma_domain::FailureKind> {
    let Some(settings_repo) = settings_repo else {
        return Err(luma_domain::FailureKind::Unavailable {
            reason: "no settings repository".into(),
            retryable: false,
        });
    };
    let current =
        settings_repo
            .load_or_default()
            .map_err(|err| luma_domain::FailureKind::Unavailable {
                reason: err.to_string(),
                retryable: true,
            })?;
    let expected_version = current.settings_version;
    let mut next = current.clone();
    if let Some(path) = patch.get("import_project").and_then(|v| v.as_str()) {
        next.import_project_path(std::path::Path::new(path))
            .map_err(|err| luma_domain::FailureKind::SecurityDenied { reason: err })?;
    } else if let Some(name) = patch.get("remove_project").and_then(|v| v.as_str()) {
        next.remove_imported_project(name)
            .map_err(|err| luma_domain::FailureKind::NotFound { entity: err })?;
    } else {
        return Err(luma_domain::FailureKind::InvalidInput {
            field: "patch".into(),
            message: "unsupported settings mutation".into(),
        });
    }
    let roots_changed = next.imported_projects != current.imported_projects
        || next.projects_roots != current.projects_roots
        || next.notes_root != current.notes_root
        || next.records_root != current.records_root
        || next.notes_exclude_patterns != current.notes_exclude_patterns;
    let saved = settings_repo
        .update_cas(expected_version, next)
        .map_err(|err| luma_domain::FailureKind::Conflict {
            reason: err.to_string(),
        })?;
    if roots_changed {
        let modules = {
            let g = inner.lock().await;
            g.registry.enabled_modules().into_iter().collect::<Vec<_>>()
        };
        for module in modules {
            module.apply_settings(&saved).await;
        }
    }
    let rows = {
        let g = inner.lock().await;
        g.registry.list()
    };
    let tx = inner.lock().await.event_broadcast_tx.clone();
    let _ = tx.send(Event::SettingsChanged {
        version: saved.settings_version,
        settings: serde_json::json!({
            "source": "config_store",
            "modules": rows.iter().map(|(id, enabled, name)| {
                serde_json::json!({"id": id, "enabled": enabled, "name": name})
            }).collect::<Vec<_>>(),
            "notes_root": saved.notes_root,
            "projects_roots": saved.projects_roots,
            "imported_projects": saved.imported_projects,
            "notes_exclude_patterns": saved.notes_exclude_patterns,
        }),
    });
    Ok("settings updated".into())
}

/// Drain lagged frames until the next event or channel close.
async fn recv_event(rx: &mut broadcast::Receiver<Event>) -> Option<Event> {
    loop {
        match rx.recv().await {
            Ok(ev) => return Some(ev),
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(broadcast::error::RecvError::Closed) => return None,
        }
    }
}

/// One-shot helper for non-interactive CLI: own engine lifecycle for a single query.
pub async fn run_query(
    registry: ModuleRegistry,
    query: &str,
    settings: Option<Arc<dyn crate::ports::SettingsRepository>>,
) -> Result<(Vec<SearchItemDto>, Vec<Event>), String> {
    let triggers = registry.all_triggers();
    let query = luma_domain::Query::normalize_for_cli(query, |token| {
        is_meta_prefix(token) || triggers.iter().any(|t| t == token)
    });
    let engine = Engine::with_settings(registry, settings);
    let mut rx = engine.subscribe();

    engine.start_session().await;
    let request_id = "cli-1".to_string();
    let search = engine.handle_command(Command::Search {
        request_id: request_id.clone(),
        query,
    });

    let collect = async {
        let mut events = Vec::new();
        let mut items: HashMap<String, SearchItemDto> = HashMap::new();
        while let Some(ev) = recv_event(&mut rx).await {
            match &ev {
                Event::ResultsChunk {
                    upserts,
                    removed_ids,
                    ..
                } => {
                    for u in upserts {
                        items.insert(u.id.clone(), u.clone());
                    }
                    for id in removed_ids {
                        items.remove(id);
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
    settings: Option<Arc<dyn crate::ports::SettingsRepository>>,
) -> Result<(SearchItemDto, luma_protocol::ActionOutcomeDto), String> {
    let triggers = registry.all_triggers();
    let query = luma_domain::Query::normalize_for_cli(query, |token| {
        is_meta_prefix(token) || triggers.iter().any(|t| t == token)
    });
    let engine = Engine::with_settings(registry, settings);
    let mut rx = engine.subscribe();
    engine.start_session().await;
    let request_id = "cli-action-search".to_string();
    let search = engine.handle_command(Command::Search { request_id, query });
    let collect = async {
        let mut items = HashMap::new();
        while let Some(event) = recv_event(&mut rx).await {
            match event {
                Event::ResultsChunk {
                    upserts,
                    removed_ids,
                    ..
                } => {
                    for item in upserts {
                        items.insert(item.id.clone(), item);
                    }
                    for id in removed_ids {
                        items.remove(&id);
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
                .iter()
                .find(|item| {
                    !matches!(
                        item.kind.as_str(),
                        "warming"
                            | "unavailable"
                            | "not_configured"
                            | "onboarding"
                            | "status"
                            | "permission_required"
                    ) && item.primary_action_id.as_str() != "noop"
                })
                .cloned()
                .ok_or_else(|| "query returned no actionable results".to_string())?
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
        match recv_event(&mut rx).await {
            Some(Event::ActionFinished { outcome, .. }) => break outcome,
            Some(_) => {}
            None => return Err("engine event channel closed".into()),
        }
    };
    engine.handle_command(Command::ShutdownSession).await;
    Ok((selected, outcome))
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
        let mut events = engine.subscribe();
        engine.start_session().await;
        engine
            .handle_command(Command::Search {
                request_id: "sticky-1".into(),
                query: "hello".into(),
            })
            .await;
        while !matches!(events.recv().await, Ok(Event::SearchStarted { .. })) {}
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
        let mut events = engine.subscribe();
        engine.start_session().await;
        while !matches!(events.recv().await, Ok(Event::SessionReady { .. })) {}
        // Cancel arrives before Search for the same request id.
        engine
            .handle_command(Command::CancelSearch {
                request_id: "early".into(),
            })
            .await;
        assert!(matches!(
            events.recv().await,
            Ok(Event::SearchCancelled { request_id }) if request_id == "early"
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
        let mut events = engine.subscribe();
        engine.start_session().await;

        engine
            .handle_command(Command::Search {
                request_id: "old".into(),
                query: "hello".into(),
            })
            .await;
        while !matches!(events.recv().await, Ok(Event::SearchStarted { .. })) {}

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
                        Ok(Event::SearchCancelled { request_id }) if request_id == "new" => {
                            saw_new_cancelled = true;
                            break;
                        }
                        Ok(Event::SearchFinished { request_id, .. }) if request_id == "new" => {
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
        let (items, _events) = run_query(fake_registry(), "hello", None).await.unwrap();
        assert_eq!(items.len(), 1);
        assert!(items[0].title.contains("hello"));
    }

    #[tokio::test]
    async fn run_action_executes_fake_result() {
        let (result, outcome) = run_action(fake_registry(), "hello", None, "open", false, None)
            .await
            .unwrap();
        assert_eq!(result.id, "fake-1");
        assert!(matches!(
            outcome,
            luma_protocol::ActionOutcomeDto::Success { .. }
        ));
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
        let mut events = engine.subscribe();
        engine.start_session().await;
        engine
            .handle_command(Command::Search {
                request_id: "r1".into(),
                query: "wait hello".into(),
            })
            .await;
        while !matches!(events.recv().await, Ok(Event::SearchFinished { .. })) {}
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
        while !matches!(events.recv().await, Ok(Event::ActionStarted { .. })) {}
        engine
            .handle_command(Command::CancelOperation {
                operation_id: "op1".into(),
            })
            .await;
        let outcome = loop {
            if let Ok(Event::ActionFinished { outcome, .. }) = events.recv().await {
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
        let mut events = engine.subscribe();
        engine.start_session().await;
        engine
            .handle_command(Command::Search {
                request_id: "r1".into(),
                query: "wait hello".into(),
            })
            .await;
        while !matches!(events.recv().await, Ok(Event::SearchFinished { .. })) {}
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
        while !matches!(events.recv().await, Ok(Event::ActionStarted { .. })) {}
        engine
            .handle_command(Command::SetModuleEnabled {
                module_id: "luma.wait".into(),
                enabled: false,
            })
            .await;
        let outcome = loop {
            if let Ok(Event::ActionFinished { outcome, .. }) = events.recv().await {
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
        let mut events = engine.subscribe();
        engine
            .handle_command(Command::UpdateSettings {
                patch: serde_json::json!({"enabled_modules": {"luma.fake": false}}),
                expected_version: 1,
            })
            .await;
        let event = loop {
            if let Ok(Event::SettingsChanged { version, settings }) = events.recv().await {
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
    async fn import_project_cas_conflict_does_not_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(luma_storage::ConfigStore::with_path(
            dir.path().join("settings.toml"),
        ));
        let settings = Arc::new(crate::TomlSettingsRepository::new(store.clone()));
        let engine = Engine::with_settings(fake_registry(), Some(settings));
        let mut events = engine.subscribe();

        let proj1 = dir.path().join("proj1");
        let proj2 = dir.path().join("proj2");
        std::fs::create_dir(&proj1).unwrap();
        std::fs::create_dir(&proj2).unwrap();

        engine
            .handle_command(Command::UpdateSettings {
                patch: serde_json::json!({"import_project": proj1.display().to_string()}),
                expected_version: 1,
            })
            .await;
        loop {
            if let Ok(Event::SettingsChanged { .. }) = events.recv().await {
                break;
            }
        }

        let mut current = store.load_or_default().unwrap();
        current.enabled_modules.insert("luma.fake".into(), false);
        store
            .update_cas(current.settings_version, current)
            .expect("bump version");

        engine
            .handle_command(Command::UpdateSettings {
                patch: serde_json::json!({"import_project": proj2.display().to_string()}),
                expected_version: 1,
            })
            .await;
        let mut saw_conflict = false;
        for _ in 0..5 {
            if let Ok(Event::DiagnosticRaised { diagnostic }) = events.recv().await {
                if diagnostic.get("settings_update").and_then(|v| v.as_str()) == Some("failed") {
                    saw_conflict = true;
                    break;
                }
            }
        }
        assert!(saw_conflict, "expected settings conflict diagnostic");

        let loaded = store.load_or_default().unwrap();
        assert_eq!(loaded.imported_projects.len(), 1);
        assert!(loaded.imported_projects[0].path.contains("proj1"));
        assert!(!loaded
            .imported_projects
            .iter()
            .any(|p| p.path.contains("proj2")));
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
        let mut events = engine.subscribe();
        engine.start_session().await;
        while !matches!(events.recv().await, Ok(Event::SessionReady { .. })) {}
        engine
            .handle_command(Command::Search {
                request_id: "r-rm".into(),
                query: "rm x".into(),
            })
            .await;
        while !matches!(events.recv().await, Ok(Event::SearchFinished { .. })) {}

        engine
            .handle_command(Command::ListActions {
                result_id: "ephemeral-1".into(),
            })
            .await;
        let actions = loop {
            if let Ok(Event::ActionsAvailable { actions, .. }) = events.recv().await {
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
            if let Ok(Event::ActionFinished { outcome, .. }) = events.recv().await {
                break outcome;
            }
        };
        assert!(
            matches!(outcome, luma_protocol::ActionOutcomeDto::Failed { .. }),
            "removed id must not execute: {outcome:?}"
        );
    }

    #[tokio::test]
    async fn refresh_wordbook_review_stats_emits_event() {
        use crate::ports::WordbookRepository;
        use crate::{MemoryWordbookRepository, WordContentInput};
        use std::sync::Arc;
        let store = Arc::new(MemoryWordbookRepository::new());
        store
            .upsert_content(&WordContentInput {
                term: "alpha".into(),
                phonetic: "".into(),
                meaning: "a".into(),
                example: "".into(),
                category: "".into(),
            })
            .unwrap();
        let engine = Engine::with_options(
            fake_registry(),
            EngineOptions {
                settings: None,
                wordbook: Some(store),
                command_recipes: None,
            },
        );
        let mut events = engine.subscribe();
        engine
            .handle_command(Command::RefreshWordbookReviewStats)
            .await;
        let event = loop {
            if let Ok(Event::WordbookReviewStatsUpdated { stats }) = events.recv().await {
                break stats;
            }
        };
        assert!(event.goal >= 1);
    }

    #[tokio::test]
    async fn load_wordbook_review_registers_gradeable_result() {
        use crate::ports::WordbookRepository;
        use crate::{MemoryWordbookRepository, WordContentInput};
        use std::sync::Arc;

        let store = Arc::new(MemoryWordbookRepository::new());
        store
            .upsert_content(&WordContentInput {
                term: "alpha".into(),
                phonetic: "".into(),
                meaning: "a".into(),
                example: "".into(),
                category: "".into(),
            })
            .unwrap();
        let word_id = store.get_by_term("alpha").unwrap().unwrap().id;

        let mut registry = fake_registry();
        registry
            .register(Arc::new(FakeModule {
                manifest: ModuleManifest {
                    id: ModuleId::new("luma.wordbook"),
                    display_name: "Wordbook".into(),
                    triggers: vec!["wb".into()],
                    default_enabled: true,
                    search_mode: SearchMode::TargetedOnly,
                    required_capabilities: vec![],
                    workbench: Default::default(),
                },
                wait_for_cancel: false,
            }))
            .unwrap();
        let engine = Engine::with_options(
            registry,
            EngineOptions {
                settings: None,
                wordbook: Some(store),
                command_recipes: None,
            },
        );
        let mut events = engine.subscribe();

        engine
            .handle_command(Command::LoadWordbookReview {
                queue: "new".into(),
            })
            .await;
        let loaded = loop {
            if let Ok(Event::WordbookReviewLoaded { words, .. }) = events.recv().await {
                break words;
            }
        };
        assert_eq!(loaded[0].id, word_id);

        engine
            .handle_command(Command::ExecuteAction {
                operation_id: "op-review".into(),
                result_id: format!("wb:{word_id}"),
                action_id: "open".into(),
                confirmation: false,
            })
            .await;
        let outcome = loop {
            if let Ok(Event::ActionFinished { outcome, .. }) = events.recv().await {
                break outcome;
            }
        };
        assert!(matches!(
            outcome,
            luma_protocol::ActionOutcomeDto::Success { .. }
        ));
    }

    #[tokio::test]
    async fn broadcast_emit_never_blocks_after_256_events() {
        use crate::port::EnginePort;
        let engine = Engine::new(fake_registry());
        let mut rx = engine.subscribe();
        // Flood without a consumer draining first — producer must not hang.
        for i in 0..320 {
            engine
                .emit(Event::DiagnosticRaised {
                    diagnostic: serde_json::json!({ "n": i }),
                })
                .await
                .unwrap();
        }
        // Subscriber may lag; must still be able to recv something or Lagged.
        let mut saw = 0usize;
        for _ in 0..400 {
            match rx.try_recv() {
                Ok(_) => saw += 1,
                Err(broadcast::error::TryRecvError::Lagged(n)) => {
                    saw += n as usize;
                }
                Err(broadcast::error::TryRecvError::Empty) => break,
                Err(broadcast::error::TryRecvError::Closed) => break,
            }
        }
        assert!(saw > 0, "subscriber should observe flood or lag");
        // Engine still accepts commands after flood.
        engine.start_session().await;
        assert!(matches!(
            rx.recv().await,
            Ok(Event::ModuleStateChanged { .. }) | Ok(Event::SessionReady { .. })
        ));
    }

    #[tokio::test]
    async fn disable_module_purges_results_and_rejects_actions() {
        let engine = Arc::new(Engine::new(fake_registry()));
        let mut events = engine.subscribe();
        engine.start_session().await;
        while !matches!(events.recv().await, Ok(Event::SessionReady { .. })) {}
        engine
            .handle_command(Command::Search {
                request_id: "r1".into(),
                query: "hello".into(),
            })
            .await;
        while !matches!(events.recv().await, Ok(Event::SearchFinished { .. })) {}

        engine
            .handle_command(Command::SetModuleEnabled {
                module_id: "luma.fake".into(),
                enabled: false,
            })
            .await;
        // Drain module-disabled and results purge events.
        let mut saw_removed = false;
        for _ in 0..20 {
            match tokio::time::timeout(std::time::Duration::from_millis(50), events.recv()).await {
                Ok(Ok(Event::ResultsChunk { removed_ids, .. })) if !removed_ids.is_empty() => {
                    saw_removed = true;
                    break;
                }
                Ok(Ok(_)) => continue,
                _ => break,
            }
        }
        assert!(
            saw_removed,
            "disable must emit removed_ids for cached results"
        );

        engine
            .handle_command(Command::ListActions {
                result_id: "fake-1".into(),
            })
            .await;
        let actions = loop {
            if let Ok(Event::ActionsAvailable { actions, .. }) = events.recv().await {
                break actions;
            }
        };
        assert!(actions.is_empty(), "disabled module must not list actions");

        engine
            .handle_command(Command::ExecuteAction {
                operation_id: "op-dis".into(),
                result_id: "fake-1".into(),
                action_id: "open".into(),
                confirmation: false,
            })
            .await;
        let outcome = loop {
            if let Ok(Event::ActionFinished { outcome, .. }) = events.recv().await {
                break outcome;
            }
        };
        assert!(
            matches!(outcome, luma_protocol::ActionOutcomeDto::Failed { .. }),
            "disabled module must not execute: {outcome:?}"
        );
    }

    struct SlowSearchModule {
        manifest: ModuleManifest,
    }

    #[async_trait]
    impl LumaModule for SlowSearchModule {
        fn manifest(&self) -> &ModuleManifest {
            &self.manifest
        }

        async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
            ModuleState::Ready
        }

        async fn search(&self, _query: Query, _sink: SearchSink, _cancel: CancellationToken) {
            tokio::time::sleep(Duration::from_secs(30)).await;
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

    #[tokio::test]
    async fn search_completion_bound_aborts_slow_module() {
        let mut registry = ModuleRegistry::new();
        registry
            .register(Arc::new(SlowSearchModule {
                manifest: ModuleManifest {
                    id: ModuleId::new("luma.slow"),
                    display_name: "Slow".into(),
                    triggers: vec!["slow".into()],
                    default_enabled: true,
                    search_mode: SearchMode::GlobalContributing,
                    required_capabilities: vec![],
                    workbench: Default::default(),
                },
            }))
            .expect("register slow");
        let engine = Engine::new(registry);
        let mut events = engine.subscribe();
        let started = std::time::Instant::now();
        engine
            .handle_command(Command::Search {
                request_id: "slow-1".into(),
                query: "slow hello".into(),
            })
            .await;
        let finished = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if let Ok(Event::SearchFinished { request_id, .. }) = events.recv().await {
                    if request_id == "slow-1" {
                        break;
                    }
                }
            }
        })
        .await;
        assert!(finished.is_ok(), "slow search must emit SearchFinished");
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "slow module search must not block past completion bound"
        );
    }
}
