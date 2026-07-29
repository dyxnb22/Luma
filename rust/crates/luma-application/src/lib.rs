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
    SqliteDatabasePortalsRepository, SqliteQuicklinksRepository, SqliteRecallRepository,
    SqliteRecordsRepository, SqliteRenewalsRepository, SqliteSnippetsRepository,
    SqliteSshMetaRepository, SqliteTimersRepository, SqliteWordbookRepository,
    TomlSettingsRepository,
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
    ActionOutcome, ActionRequest, CommandSpec, HubWindowRow, HubWindowsSlice, HubWindowsStatus,
    LumaModule, ModuleManifest, ModuleState, SearchMode, SearchSink, WarmupContext, WorkbenchMeta,
};
pub use paste::{paste_to_target_app, AX_PASTE_TIMEOUT, NO_PASTE_TARGET_REASON};
pub use port::EnginePort;
pub use ports::{
    filter_env_output, format_connection_subtitle, frontmost_matches_paste_target,
    is_filtered_env_step, looks_secret, mutation_args, postgres_client_args, recipe_in_scope,
    recipe_runnable, resolve_steps, sanitize_identity_display, select_best_variant,
    ssh_password_account, validate_mutation_state, validate_postgres_metadata, AccessibilityError,
    AccessibilityPort, AppEntry, AppLaunchError, AppSettings, AppsCatalogPort,
    BoundedUtf8FileReadError, BoundedUtf8FileReaderPort, CapabilityPort, ClipboardEntry,
    ClipboardHistoryRepository, ClipboardRepoError, ClockError, ClockPort, CommandRecipesRepoError,
    CommandRecipesRepository, CommandRunnerPort, ContentImportReport, ControllableClock,
    DatabaseClientPlan, DatabasePlatformError, DatabasePlatformPort, DatabasePortal,
    DatabasePortalTarget, DatabasePortalsRepoError, DatabasePortalsRepository,
    DatabaseSchemaObject, DownloadCategory, DownloadEntry, DownloadsError, DownloadsFilter,
    DownloadsPort, EmbeddedPtyError, EmbeddedPtyEvent, EmbeddedPtyPort, EmbeddedPtySession,
    EmbeddedPtySize, EmbeddedPtySpawnRequest, ExternalControllerStatus, FakeAccessibility,
    FakeBoundedUtf8FileReader, FakeCapabilities, FakeCommandRunner, FakeDatabasePlatform,
    FakeDownloads, FakeEmbeddedPty, FakeGitRepository, FakeKeychain, FakeNetworkProbe,
    FakeOpenPath, FakePackageManager, FakePasteboard, FakeProjectWorkspace, FakeProxyCore,
    FakeRecipeEnvironment, FakeRuntimePort, FakeScreenOcr, FakeShellHistory, FakeShortcuts,
    FakeSpeech, FakeSshConfigPort, FakeSystemProxy, FakeSystemSettings, FakeWindowCatalog,
    FixedClock, GitBranch, GitCommit, GitDiff, GitError, GitFile, GitProjectRoot,
    GitRepositoryPort, GitRepositoryState, KeychainError, KeychainPort, MemoryClipboardHistory,
    MemoryDatabasePortalsRepository, MemoryQuicklinksRepository, MemoryRecordsRepository,
    MemoryRenewalsRepository, MemorySnippetsRepository, MemorySshMetaRepository,
    MemoryTimersRepository, MemoryWordbookRepository, NetworkProbePort, NetworkProbeState,
    NetworkProbeStep, NewDatabasePortal, NewRenewal, OpenPathError, OpenPathPort, PackageError,
    PackageKind, PackageManagerPort, PackageMutation, PackageMutationPlan, PackageQuery,
    PackageRecord, PasteboardError, PasteboardPort, PasteboardSnapshot, PathKind,
    ProfileImportResult, ProfileSource, ProfileStoreError, ProfileStorePort, ProfileSummary,
    ProjectDirectoryEntry, ProjectDirectoryListing, ProjectOpenScope, ProjectWorkspaceError,
    ProjectWorkspacePort, ProxyCoreError, ProxyCorePort, ProxyGroup, ProxyMode, ProxyNode,
    ProxyPorts, ProxyStatus, QuicklinkEntry, QuicklinksRepoError, QuicklinksRepository,
    RecallObject, RecallRepoError, RecallRepository, RecipeEnvironmentError,
    RecipeEnvironmentPort, RecipeStdioMode, RecordCategory, RecordEntry, RecordImportPreviewView,
    RecordImportReportView, RecordsRepoError, RecordsRepository, RecordsStatsView, RenewalEntry,
    RenewalPaidUpdate, RenewalsRepoError, RenewalsRepository, ResolvedSshHost, RuntimeError,
    RuntimeListener, RuntimePort, ScreenOcrError, ScreenOcrPort, SecretLabel, SettingsError,
    SettingsRepository, ShellHistoryEntry, ShellHistoryError, ShellHistoryPort,
    ShellHistorySnapshot, ShortcutEntry, ShortcutRunPlan, ShortcutsError, ShortcutsPort,
    SnippetEntry, SnippetsRepoError, SnippetsRepository, SpeechAccent, SpeechError, SpeechPort,
    SshConfigError, SshConfigPort, SshConfigState, SshHostMeta, SshMetaRepoError,
    SshMetaRepository, SystemProxyError, SystemProxyPort, SystemProxySetting, SystemProxyStatus,
    SystemSettingsError, SystemSettingsPane, SystemSettingsPort, TimerEntry, TimersRepoError,
    TimersRepository, UnavailableNetworkProbe, WindowCatalogPort, WindowEntry, WindowError,
    WordContentInput, WordEntry, WordbookRepoError, WordbookRepository, WordbookStatsView,
    MAX_DATABASE_PORTALS, MAX_DATABASE_SCHEMA_BYTES, MAX_DATABASE_SCHEMA_OBJECTS,
    MAX_DOWNLOAD_ENTRIES, MAX_OCR_TEXT_BYTES, MAX_PACKAGE_OUTPUT_BYTES, MAX_PACKAGE_RESULTS,
    MAX_RENEWALS, MAX_SHELL_HISTORY_BYTES, MAX_SHELL_HISTORY_COMMAND_BYTES,
    MAX_SHELL_HISTORY_ENTRIES, MAX_SHORTCUT_OUTPUT_BYTES, MAX_SHORTCUT_RESULTS,
    SSH_ASKPASS_ACCOUNT_ENV, SSH_PASSWORD_ACCOUNT_PREFIX, recv_event_timeout,
};
pub use recipe_runner::{
    execute_recipe_plan, execute_recipe_plan_with_hooks, now_unix, recipe_outcome_to_action_dto,
    record_recipe_run_outcome, spawn_ctrl_c_cancel, RecipeExecuteError, RecipeExecuteOptions,
    RecipeExecuteReport,
};
pub use registry::{ModuleRegistry, RegistryError};
pub use unavailable_module::UnavailableModule;
