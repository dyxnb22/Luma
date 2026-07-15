//! Application engine. No ratatui/crossterm.

mod adapters;
mod engine;
mod module;
mod paste;
mod port;
mod ports;
mod registry;

pub use adapters::{
    SqliteClipboardHistory, SqliteNotesIndex, SqliteQuicklinksRepository, SqliteRecordsRepository,
    SqliteSnippetsRepository, SqliteWordbookRepository, TomlSettingsRepository,
};
pub use engine::{list_modules_json, run_action, run_query, Engine, EngineOptions};
pub use module::{
    ActionOutcome, ActionRequest, HubWindowRow, HubWindowsSlice, HubWindowsStatus, LumaModule,
    ModuleManifest, ModuleState, SearchMode, SearchSink, WarmupContext, WorkbenchMeta,
};
pub use paste::{paste_to_target_app, AX_PASTE_TIMEOUT, NO_PASTE_TARGET_REASON};
pub use port::EnginePort;
pub use ports::{
    looks_secret, AccessibilityError, AccessibilityPort, AppEntry, AppLaunchError, AppSettings,
    AppsCatalogPort, CapabilityPort, ClipboardEntry, ClipboardHistoryRepository,
    ClipboardRepoError, ClockError, ClockPort, ContentImportReport, ExternalControllerStatus,
    FakeAccessibility, FakeCapabilities, FakeKeychain, FakeMarkdownWatcher, FakeOpenPath,
    FakePasteboard, FakeProxyCore, FakeSpeech, FakeSystemProxy, FakeWindowCatalog, FixedClock,
    KeychainError, KeychainPort, MarkdownWatchPort, MemoryClipboardHistory, MemoryNotesIndex,
    MemoryQuicklinksRepository, MemoryRecordsRepository, MemorySnippetsRepository,
    MemoryWordbookRepository, NotesDocument, NotesIndexError, NotesIndexRepository, NotesIssue,
    NotesLink, NotesScanReport, NotesScanStatusView, NotesSearchHit, OpenPathError, OpenPathPort,
    PasteboardError, PasteboardPort, ProxyCoreError, ProxyCorePort, ProxyGroup, ProxyMode,
    ProxyNode, ProxyPorts, ProxyStatus, QuicklinkEntry, QuicklinksRepoError, QuicklinksRepository,
    RecordCategory, RecordEntry, RecordImportPreviewView, RecordImportReportView, RecordsRepoError,
    RecordsRepository, RecordsStatsView, SecretLabel, SettingsError, SettingsRepository,
    SnippetEntry, SnippetsRepoError, SnippetsRepository, SpeechAccent, SpeechError, SpeechPort,
    SystemClock, SystemProxyError, SystemProxyPort, SystemProxySetting, SystemProxyStatus,
    WindowCatalogPort, WindowEntry, WindowError, WordContentInput, WordEntry, WordbookRepoError,
    WordbookRepository, WordbookStatsView,
};
pub use registry::{ModuleRegistry, RegistryError};
// Application-facing façade for project settings helpers. Modules depend on this layer rather
// than importing storage adapters directly.
pub use luma_storage::{validate_import_project_path, ImportedProject};
