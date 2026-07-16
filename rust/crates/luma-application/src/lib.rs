//! Application engine. No ratatui/crossterm.

mod adapters;
mod engine;
mod interactive_terminal;
mod module;
mod paste;
mod port;
mod ports;
mod registry;

pub use adapters::{
    JsonResumeContextsRepository, MemoryCommandRecipesRepository, SqliteClipboardHistory,
    SqliteCommandRecipesRepository, SqliteNotesIndex, SqliteQuicklinksRepository,
    SqliteRecordsRepository, SqliteSnippetsRepository, SqliteSshMetaRepository,
    SqliteWordbookRepository, TomlSettingsRepository,
};
pub use engine::{list_modules_json, run_action, run_query, Engine, EngineOptions};
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
    blank_context, filter_env_output, format_connection_subtitle, is_filtered_env_step,
    looks_secret, recipe_in_scope, recipe_runnable, resolve_steps, sanitize_identity_display,
    select_best_variant, AccessibilityError, AccessibilityPort, AppEntry, AppLaunchError,
    AppSettings, AppsCatalogPort, BoundedUtf8FileReadError, BoundedUtf8FileReaderPort,
    CapabilityPort, ClipboardEntry, ClipboardHistoryRepository, ClipboardRepoError, ClockError,
    ClockPort, CommandRecipesRepoError, CommandRecipesRepository, CommandRunnerPort,
    ContentImportReport, ExternalControllerStatus, FakeAccessibility, FakeBoundedUtf8FileReader,
    FakeCapabilities, FakeCommandRunner, FakeEditorCall, FakeGitInfo, FakeKeychain,
    FakeMarkdownWatcher, FakeNotesWorkspace, FakeOpenEditor, FakeOpenPath, FakePasteboard,
    FakeProjectWorkspace, FakeProxyCore, FakeRecipeEnvironment, FakeSpeech, FakeSshConfigPort,
    FakeSystemProxy, FakeWindowCatalog, FixedClock, GitInfoError, GitInfoPort, GitSnapshot,
    KeychainError, KeychainPort, MarkdownWatchPort, MemoryClipboardHistory, MemoryNotesIndex,
    MemoryQuicklinksRepository, MemoryRecordsRepository, MemoryResumeContextsRepository,
    MemorySnippetsRepository, MemorySshMetaRepository, MemoryWordbookRepository,
    NotesDirectoryEntry, NotesDirectoryEntryKind, NotesDirectoryListing, NotesDocument,
    NotesIndexError, NotesIndexRepository, NotesIssue, NotesLink, NotesScanReport,
    NotesScanStatusView, NotesSearchHit, NotesWorkspaceError, NotesWorkspacePath,
    NotesWorkspacePort, NotesWorkspacePreview, OpenEditorError, OpenEditorPort, OpenPathError,
    OpenPathPort, PasteboardError, PasteboardPort, PathKind, ProfileImportResult, ProfileSource,
    ProfileStoreError, ProfileStorePort, ProfileSummary, ProjectDirectoryEntry,
    ProjectDirectoryListing, ProjectOpenScope, ProjectWorkspaceError, ProjectWorkspacePort,
    ProxyCoreError, ProxyCorePort, ProxyGroup, ProxyMode, ProxyNode, ProxyPorts, ProxyStatus,
    QuicklinkEntry, QuicklinksRepoError, QuicklinksRepository, RecipeEnvironmentError,
    RecipeEnvironmentPort, RecordCategory, RecordEntry, RecordImportPreviewView,
    RecordImportReportView, RecordsRepoError, RecordsRepository, RecordsStatsView,
    ResolvedSshHost, ResumeContextsRepository, ResumeRepoError, SecretLabel, SettingsError,
    SettingsRepository, SnippetEntry, SnippetsRepoError, SnippetsRepository, SpeechAccent,
    SpeechError, SpeechPort, SshConfigError, SshConfigPort, SshConfigState, SshHostMeta,
    SshMetaRepoError, SshMetaRepository, SystemProxyError, SystemProxyPort, SystemProxySetting,
    SystemProxyStatus, WindowCatalogPort, WindowEntry, WindowError, WordContentInput, WordEntry,
    WordbookRepoError, WordbookRepository, WordbookStatsView,
};
pub use registry::{ModuleRegistry, RegistryError};

// Re-export resume model types used by modules/adapters.
pub use luma_storage::{
    normalize_name as normalize_resume_name, normalize_path_for_store as normalize_resume_path,
    resume_now_iso, ResumeContext, ResumeEditor, ResumeRecipeRef, ResumeStore, ResumeStoreError,
};
