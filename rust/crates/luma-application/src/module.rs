use crate::ports::AppSettings;
use async_trait::async_trait;
use luma_domain::{ActionDescriptor, FailureKind, ModuleId, Query, RecipeRunPlan, SearchItem};
use luma_protocol::Event;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
    TargetedOnly,
    GlobalContributing,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WorkbenchMeta {
    /// Optional single-glyph override for the TUI (else derived from module id).
    #[serde(default)]
    pub glyph: Option<String>,
    /// Query inserted from the Hub (e.g. `"app "`).
    #[serde(default)]
    pub suggested_query: Option<String>,
    /// Empty-state hint for this module.
    #[serde(default)]
    pub empty_hint: Option<String>,
    /// Module participates in browse / drill-down queries.
    #[serde(default)]
    pub supports_browse: bool,
    /// Canonical slash-command surfaces owned by this module.
    ///
    /// Help, command discovery, and completion project this same list. `query` is the editable
    /// seed inserted by the command palette; `syntax` remains the user-facing usage string.
    #[serde(default)]
    pub commands: Vec<CommandSpec>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSpec {
    pub syntax: String,
    pub description: String,
    pub query: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub example: Option<String>,
}

impl CommandSpec {
    pub fn new(
        syntax: impl Into<String>,
        description: impl Into<String>,
        query: impl Into<String>,
    ) -> Self {
        Self {
            syntax: syntax.into(),
            description: description.into(),
            query: query.into(),
            example: None,
        }
    }

    pub fn example(mut self, example: impl Into<String>) -> Self {
        self.example = Some(example.into());
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleManifest {
    pub id: ModuleId,
    pub display_name: String,
    pub triggers: Vec<String>,
    pub default_enabled: bool,
    pub search_mode: SearchMode,
    pub required_capabilities: Vec<String>,
    #[serde(default)]
    pub workbench: WorkbenchMeta,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModuleState {
    Cold,
    Ready,
    Disabled,
    Failed(String),
}

#[derive(Clone, Debug)]
pub struct WarmupContext {
    pub cancel: CancellationToken,
}

#[derive(Clone, Debug)]
pub struct ActionRequest {
    pub result: SearchItem,
    pub action: ActionDescriptor,
    pub confirmation: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionOutcome {
    Success {
        message: Option<String>,
    },
    Failed {
        kind: FailureKind,
    },
    Cancelled,
    /// Request the client to open another slash-prefixed local surface.
    ///
    /// This remains a generic navigation result: modules describe the destination while the
    /// client owns prompt/history state. Unlike a result-only UI intent, it is an executed
    /// success and can therefore participate in Recall.
    OpenSurface {
        query: String,
    },
    /// Request TUI to run an interactive subprocess in the current terminal.
    InteractiveTerminal {
        program: String,
        args: Vec<String>,
        environment: Vec<(String, String)>,
    },
    /// Request a settings.toml CAS update (handled by the engine).
    SettingsMutation {
        patch: serde_json::Value,
    },
    /// Describe an interactive recipe run for the TUI to execute in the current terminal.
    InteractiveRecipeRun {
        plan: Box<RecipeRunPlan>,
    },
}

pub type SearchSink = mpsc::Sender<Event>;

/// Hub projection of previous-frontmost app windows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HubWindowRow {
    pub id: String,
    pub title: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HubWindowsStatus {
    pub kind: String,
    pub title: String,
    pub subtitle: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HubWindowsSlice {
    pub app_name: String,
    pub windows: Vec<HubWindowRow>,
    /// Extra windows beyond the hard cap (shown as `N more → win`).
    pub more: Option<u32>,
    /// Structured Hub failure (permission / unavailable) — never silent empty.
    pub status: Option<HubWindowsStatus>,
}

#[async_trait]
pub trait LumaModule: Send + Sync {
    fn manifest(&self) -> &ModuleManifest;

    async fn warmup(&self, ctx: WarmupContext) -> ModuleState;

    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken);

    async fn actions(&self, result: &SearchItem) -> Vec<ActionDescriptor>;

    /// Optional detail body for the workbench preview pane.
    async fn preview(&self, result: &SearchItem) -> Option<String> {
        result
            .subtitle
            .clone()
            .or_else(|| Some(result.title.clone()))
    }

    /// Cheap capability flag: LoadHub only awaits [`hub_windows`] when true.
    fn supports_hub_windows(&self) -> bool {
        false
    }

    /// Optional Hub windows slice (previous-frontmost app). Default: none.
    async fn hub_windows(&self) -> Option<HubWindowsSlice> {
        None
    }

    /// Optional live Hub objects such as running timers. These use the same natural-action
    /// contract as Continue rows but are derived from current module state, not persistence.
    fn supports_hub_items(&self) -> bool {
        false
    }

    async fn hub_items(&self, _limit: usize) -> Vec<SearchItem> {
        Vec::new()
    }

    /// Re-read a persisted Recall identity from the module's current source of truth.
    ///
    /// `Ok(None)` means the object is permanently stale and its Recall row may be removed.
    /// `Err` means revalidation was temporarily unavailable, so persistence must be retained.
    /// The default keeps modules out of Hub Continue until they can prove a safe live identity.
    async fn rehydrate_recall(&self, _object_id: &str) -> Result<Option<SearchItem>, String> {
        Ok(None)
    }

    /// Apply settings that change at runtime (roots, excludes). Default: no-op.
    async fn apply_settings(&self, _settings: &AppSettings) {}

    async fn perform(&self, action: ActionRequest, cancel: CancellationToken) -> ActionOutcome;

    async fn teardown(&self);
}
