//! Application engine. No ratatui/crossterm.

mod adapters;
mod engine;
mod module;
mod port;
mod ports;
mod registry;

pub use adapters::{
    FsDiagnosticsSink, SqliteClipboardHistory, SqliteNotesIndex, SqliteQuicklinksRepository,
    SqliteSnippetsRepository, TomlSettingsRepository,
};
pub use engine::{list_modules_json, run_action, run_doctor, run_query, Engine, EngineOptions};
pub use module::{
    ActionOutcome, ActionRequest, HubWindowRow, HubWindowsSlice, HubWindowsStatus, LumaModule,
    ModuleManifest, ModuleState, SearchMode, SearchSink, WarmupContext, WorkbenchMeta,
};
pub use port::EnginePort;
pub use ports::{
    looks_secret, AccessibilityError, AccessibilityPort, AppEntry, AppLaunchError, AppSettings,
    AppsCatalogPort, ClipboardEntry, ClipboardHistoryRepository, ClipboardRepoError, ClockError,
    ClockPort, DiagnosticsError, DiagnosticsSink, FakeAccessibility, FakeKeychain, FakeOpenPath,
    FakePasteboard, FakeWindowCatalog, FixedClock, KeychainError, KeychainPort, MarkdownWatchPort,
    MemoryClipboardHistory, MemoryNotesIndex, MemoryQuicklinksRepository, MemorySnippetsRepository,
    NotesDocument, NotesIndexError, NotesIndexRepository, NotesIssue, NotesLink, NotesScanReport,
    NotesScanStatusView, NotesSearchHit, OpenPathError, OpenPathPort, PasteboardError,
    PasteboardPort, QuicklinkEntry, QuicklinksRepoError, QuicklinksRepository, SecretLabel,
    SettingsError, SettingsRepository, SnippetEntry, SnippetsRepoError, SnippetsRepository,
    SystemClock, WindowCatalogPort, WindowEntry, WindowError,
};
pub use registry::{ModuleRegistry, RegistryError};
