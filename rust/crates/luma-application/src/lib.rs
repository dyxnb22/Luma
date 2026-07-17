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

pub use adapters::{
    MemoryCommandRecipesRepository, SqliteClipboardHistory, SqliteCommandRecipesRepository,
    SqliteNotesIndex, SqliteQuicklinksRepository, SqliteRecordsRepository,
    SqliteSnippetsRepository, SqliteSshMetaRepository, SqliteTimersRepository,
    SqliteWordbookRepository, TomlSettingsRepository,
};
pub use engine::{
    list_modules_json, run_action, run_query, Engine, EngineOptions, RunActionOptions,
};
pub use interactive_terminal::{
    run_interactive_terminal, sftp_args, ssh_connect_args, InteractiveTerminalError,
    InteractiveTerminalRequest,
};
pub use luma_storage::ImportedProject;
pub use module::{
    ActionOutcome, ActionRequest, HubWindowRow, HubWindowsSlice, HubWindowsStatus, LumaModule,
    ModuleManifest, ModuleState, SearchMode, SearchSink, WarmupContext, WorkbenchMeta,
};
pub use paste::{paste_to_target_app, AX_PASTE_TIMEOUT, NO_PASTE_TARGET_REASON};
pub use port::EnginePort;
pub use ports::{
    filter_env_output, format_connection_subtitle, frontmost_matches_paste_target,
    is_filtered_env_step, looks_secret, recipe_in_scope, recipe_runnable, resolve_steps,
    sanitize_identity_display, select_best_variant, AccessibilityError, AccessibilityPort,
    AppEntry, AppLaunchError, AppSettings, AppsCatalogPort, BoundedUtf8FileReadError,
    BoundedUtf8FileReaderPort, CapabilityPort, ClipboardEntry, ClipboardHistoryRepository,
    ClipboardRepoError, ClockError, ClockPort, CommandRecipesRepoError, CommandRecipesRepository,
    CommandRunnerPort, ContentImportReport, ControllableClock, ExternalControllerStatus,
    FakeAccessibility, FakeBoundedUtf8FileReader, FakeCapabilities, FakeCommandRunner,
    FakeKeychain, FakeMarkdownWatcher, FakeNotesWorkspace, FakeOpenPath, FakePasteboard,
    FakeProjectWorkspace, FakeProxyCore, FakeRecipeEnvironment, FakeSpeech, FakeSshConfigPort,
    FakeSystemProxy, FakeWindowCatalog, FixedClock, KeychainError, KeychainPort, MarkdownWatchPort,
    MemoryClipboardHistory, MemoryNotesIndex, MemoryQuicklinksRepository, MemoryRecordsRepository,
    MemorySnippetsRepository, MemorySshMetaRepository, MemoryTimersRepository,
    MemoryWordbookRepository, NotesDirectoryEntry, NotesDirectoryEntryKind, NotesDirectoryListing,
    NotesDocument, NotesIndexError, NotesIndexRepository, NotesIssue, NotesLink, NotesScanReport,
    NotesScanStatusView, NotesSearchHit, NotesWorkspaceError, NotesWorkspacePath,
    NotesWorkspacePort, NotesWorkspacePreview, OpenPathError, OpenPathPort, PasteboardError,
    PasteboardPort, PathKind, ProfileImportResult, ProfileSource, ProfileStoreError,
    ProfileStorePort, ProfileSummary, ProjectDirectoryEntry, ProjectDirectoryListing,
    ProjectOpenScope, ProjectWorkspaceError, ProjectWorkspacePort, ProxyCoreError, ProxyCorePort,
    ProxyGroup, ProxyMode, ProxyNode, ProxyPorts, ProxyStatus, QuicklinkEntry, QuicklinksRepoError,
    QuicklinksRepository, RecipeEnvironmentError, RecipeEnvironmentPort, RecipeStdioMode,
    RecordCategory, RecordEntry, RecordImportPreviewView, RecordImportReportView, RecordsRepoError,
    RecordsRepository, RecordsStatsView, ResolvedSshHost, SecretLabel, SettingsError,
    SettingsRepository, SnippetEntry, SnippetsRepoError, SnippetsRepository, SpeechAccent,
    SpeechError, SpeechPort, SshConfigError, SshConfigPort, SshConfigState, SshHostMeta,
    SshMetaRepoError, SshMetaRepository, SystemProxyError, SystemProxyPort, SystemProxySetting,
    SystemProxyStatus, TimerEntry, TimersRepoError, TimersRepository, WindowCatalogPort,
    WindowEntry, WindowError, WordContentInput, WordEntry, WordbookRepoError, WordbookRepository,
    WordbookStatsView,
};
pub use recipe_runner::{
    execute_recipe_plan, execute_recipe_plan_with_hooks, now_unix, recipe_outcome_to_action_dto,
    record_recipe_run_outcome, spawn_ctrl_c_cancel, RecipeExecuteError, RecipeExecuteOptions,
    RecipeExecuteReport,
};
pub use registry::{ModuleRegistry, RegistryError};
