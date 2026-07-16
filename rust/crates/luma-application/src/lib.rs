//! Application engine. No ratatui/crossterm.

mod adapters;
mod engine;
mod module;
mod paste;
mod port;
mod ports;
mod registry;

pub use adapters::{
    MemoryCommandRecipesRepository, SqliteClipboardHistory, SqliteCommandRecipesRepository,
    SqliteNotesIndex, SqliteQuicklinksRepository, SqliteRecordsRepository,
    SqliteSnippetsRepository, SqliteWordbookRepository, TomlSettingsRepository,
};
pub use engine::{list_modules_json, run_action, run_query, Engine, EngineOptions};
pub use luma_storage::ImportedProject;
pub use module::{
    ActionOutcome, ActionRequest, HubWindowRow, HubWindowsSlice, HubWindowsStatus, LumaModule,
    ModuleManifest, ModuleState, SearchMode, SearchSink, WarmupContext, WorkbenchMeta,
};
pub use paste::{paste_to_target_app, AX_PASTE_TIMEOUT, NO_PASTE_TARGET_REASON};
pub use port::EnginePort;
pub use ports::{
    filter_env_output, looks_secret, resolve_steps, select_best_variant, AccessibilityError,
    AccessibilityPort, AppEntry, AppLaunchError, AppSettings, AppsCatalogPort,
    BoundedUtf8FileReadError, BoundedUtf8FileReaderPort, CapabilityPort, ClipboardEntry,
    ClipboardHistoryRepository, ClipboardRepoError, ClockError, ClockPort, CommandRecipesRepoError,
    CommandRecipesRepository, CommandRunnerPort, ContentImportReport, ExternalControllerStatus,
    FakeAccessibility, FakeBoundedUtf8FileReader, FakeCapabilities, FakeCommandRunner,
    FakeKeychain, FakeMarkdownWatcher, FakeNotesWorkspace, FakeOpenPath, FakePasteboard,
    FakeProjectWorkspace, FakeProxyCore, FakeRecipeEnvironment, FakeSpeech, FakeSystemProxy,
    FakeWindowCatalog, FixedClock, KeychainError, KeychainPort, MarkdownWatchPort,
    MemoryClipboardHistory, MemoryNotesIndex, MemoryQuicklinksRepository, MemoryRecordsRepository,
    MemorySnippetsRepository, MemoryWordbookRepository, NotesDirectoryEntry,
    NotesDirectoryEntryKind, NotesDirectoryListing, NotesDocument, NotesIndexError,
    NotesIndexRepository, NotesIssue, NotesLink, NotesScanReport, NotesScanStatusView,
    NotesSearchHit, NotesWorkspaceError, NotesWorkspacePath, NotesWorkspacePort,
    NotesWorkspacePreview, OpenPathError, OpenPathPort, PasteboardError, PasteboardPort, PathKind,
    ProfileImportResult, ProfileSource, ProfileStoreError, ProfileStorePort, ProfileSummary,
    ProjectDirectoryEntry, ProjectDirectoryListing, ProjectOpenScope, ProjectWorkspaceError,
    ProjectWorkspacePort, ProxyCoreError, ProxyCorePort, ProxyGroup, ProxyMode, ProxyNode,
    ProxyPorts, ProxyStatus, QuicklinkEntry, QuicklinksRepoError, QuicklinksRepository,
    RecipeEnvironmentError, RecipeEnvironmentPort, RecordCategory, RecordEntry,
    RecordImportPreviewView, RecordImportReportView, RecordsRepoError, RecordsRepository,
    RecordsStatsView, SecretLabel, SettingsError, SettingsRepository, SnippetEntry,
    SnippetsRepoError, SnippetsRepository, SpeechAccent, SpeechError, SpeechPort, SystemProxyError,
    SystemProxyPort, SystemProxySetting, SystemProxyStatus, WindowCatalogPort, WindowEntry,
    WindowError, WordContentInput, WordEntry, WordbookRepoError, WordbookRepository,
    WordbookStatsView,
};
pub use registry::{ModuleRegistry, RegistryError};
