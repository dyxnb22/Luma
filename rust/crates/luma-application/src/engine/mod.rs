use crate::module::{LumaModule, ModuleState, WarmupContext};
use crate::port::EnginePort;
use crate::registry::ModuleRegistry;
use async_trait::async_trait;
use luma_domain::{Query, QueryScope};
use luma_protocol::{Command, Event, SearchItemDto};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio::task::{JoinHandle, JoinSet};
use tokio_util::sync::CancellationToken;
use tracing::warn;

const SEARCH_CANCEL_BOUND: Duration = Duration::from_millis(750);
const OPERATION_CANCEL_BOUND: Duration = Duration::from_millis(750);
/// Cap concurrent in-flight ExecuteAction tasks.
pub(crate) const MAX_OPERATIONS: usize = 32;
/// Soft bound for module search completion; partial results are kept.
#[cfg(test)]
pub(crate) const SEARCH_COMPLETION_BOUND: Duration = Duration::from_millis(300);
#[cfg(not(test))]
pub(crate) const SEARCH_COMPLETION_BOUND: Duration = Duration::from_secs(5);

fn is_meta_prefix(token: &str) -> bool {
    matches!(token, "help")
}

// Freeze: do not add new module-specific Command arms here — see `extensions/` + GOVERNANCE §2.7a.
mod actions;
mod cancel_intents;
mod cli;
mod command_dispatch;
mod extensions;
mod preview;
mod results;
mod search;
mod session;

pub use cli::{list_modules_json, run_action, run_query, RunActionOptions};

struct SearchTask {
    cancel: CancellationToken,
    module_cancels: HashMap<String, CancellationToken>,
    /// Owns module search tasks + result collector; cancel awaits this with a bound.
    handle: JoinHandle<()>,
}

struct OperationTask {
    cancel: CancellationToken,
    module_id: String,
    /// Distinguishes a replacement that reuses the same public operation id.
    generation: u64,
    handle: Option<JoinHandle<()>>,
}

pub(crate) struct EngineInner {
    registry: ModuleRegistry,
    event_broadcast_tx: broadcast::Sender<Event>,
    session_cancel: CancellationToken,
    searches: HashMap<String, SearchTask>,
    /// Cancel arrived before Search registered — Search must abort on start.
    cancel_intents: HashMap<String, Instant>,
    /// Search registered under lifecycle but not yet promoted to `searches`.
    pending_searches: HashMap<String, CancellationToken>,
    operations: HashMap<String, OperationTask>,
    /// Insertion order for FIFO eviction when `operations` hits `MAX_OPERATIONS`.
    operation_order: VecDeque<String>,
    /// Monotonic identity for operation lifetimes, independent of caller-provided ids.
    next_operation_generation: u64,
    /// Newest LoadPreview id; stale preview work skips emit.
    latest_preview_id: u64,
    results_by_id: HashMap<String, luma_domain::SearchItem>,
    /// Insertion order for LRU eviction of cached search results.
    result_order: VecDeque<String>,
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
                operation_order: VecDeque::new(),
                next_operation_generation: 0,
                latest_preview_id: 0,
                results_by_id: HashMap::new(),
                result_order: VecDeque::new(),
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
mod tests;
