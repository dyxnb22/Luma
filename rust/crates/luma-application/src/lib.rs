//! Application engine. No ratatui/crossterm.

mod adapters;
mod engine;
mod interactive_terminal;
mod module;
mod paste;
mod port;
mod ports;
mod recipe_runner;
mod registry;
mod unavailable_module;

pub use adapters::{
    MemoryCommandRecipesRepository, SqliteClipboardHistory, SqliteCommandRecipesRepository,
    SqliteRecallRepository, SqliteRecordsRepository, SqliteWordbookRepository,
    TomlSettingsRepository,
};
pub use engine::{
    list_modules_json, run_action, run_query, Engine, EngineOptions, RunActionOptions,
};
pub use interactive_terminal::{
    run_interactive_terminal, InteractiveTerminalError, InteractiveTerminalRequest,
};
pub use luma_storage::ImportedProject;
pub use module::{
    ActionOutcome, ActionRequest, CommandSpec, HubWindowRow, HubWindowsSlice, HubWindowsStatus,
    LumaModule, ModuleManifest, ModuleState, SearchMode, SearchSink, WarmupContext, WorkbenchMeta,
};
pub use paste::{paste_to_target_app, AX_PASTE_TIMEOUT, NO_PASTE_TARGET_REASON};
pub use port::EnginePort;
pub use ports::{
    filter_env_output, frontmost_matches_paste_target, is_filtered_env_step, looks_secret,
    recipe_in_scope, recipe_runnable, resolve_steps, select_best_variant, AccessibilityError,
    AccessibilityPort, AppEntry, AppLaunchError, AppSettings, AppsCatalogPort,
    BoundedUtf8FileReadError, BoundedUtf8FileReaderPort, CapabilityPort, ClipboardEntry,
    ClipboardHistoryRepository, ClipboardRepoError, CommandRecipesRepoError,
    CommandRecipesRepository, CommandRunnerPort, ContentImportReport, FakeAccessibility,
    FakeBoundedUtf8FileReader, FakeCapabilities, FakeCommandRunner, FakeGitRepository,
    FakeOpenPath, FakePasteboard, FakeProjectWorkspace, FakeRecipeEnvironment, FakeRuntimePort,
    FakeSpeech, FakeSystemSettings, FakeWindowCatalog, GitBranch, GitCommit, GitDiff, GitError,
    GitFile, GitProjectRoot, GitRepositoryPort, GitRepositoryState, MemoryClipboardHistory,
    MemoryRecordsRepository, MemoryWordbookRepository, OpenPathError, OpenPathPort,
    PasteboardError, PasteboardPort, PasteboardSnapshot, PathKind, ProjectDirectoryEntry,
    ProjectDirectoryListing, ProjectOpenScope, ProjectWorkspaceError, ProjectWorkspacePort,
    RecallObject, RecallRepoError, RecallRepository, RecipeEnvironmentError, RecipeEnvironmentPort,
    RecipeStdioMode, RecordCategory, RecordEntry, RecordImportPreviewView, RecordImportReportView,
    RecordsRepoError, RecordsRepository, RecordsStatsView, RuntimeError, RuntimeListener,
    RuntimePort, SettingsError, SettingsRepository, SpeechAccent, SpeechError, SpeechPort,
    SystemSettingsError, SystemSettingsPane, SystemSettingsPort, WindowCatalogPort, WindowEntry,
    WindowError, WordContentInput, WordEntry, WordbookRepoError, WordbookRepository,
    WordbookStatsView,
};
pub use recipe_runner::{
    execute_recipe_plan, execute_recipe_plan_with_hooks, now_unix, recipe_outcome_to_action_dto,
    record_recipe_run_outcome, spawn_ctrl_c_cancel, RecipeExecuteError, RecipeExecuteOptions,
    RecipeExecuteReport,
};
pub use registry::{ModuleRegistry, RegistryError};
pub use unavailable_module::UnavailableModule;
