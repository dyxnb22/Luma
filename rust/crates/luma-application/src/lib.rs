//! Application engine. No ratatui/crossterm.

mod adapters;
mod engine;
mod module;
mod paste;
mod port;
mod ports;
mod registry;

pub use adapters::{
    FsDiagnosticsSink, SqliteClipboardHistory, SqliteNotesIndex, SqliteQuicklinksRepository,
    SqliteSnippetsRepository, TomlSettingsRepository,
};
pub use engine::{
    list_modules_json, run_action, run_doctor, run_doctor_with_options, run_query, Engine,
    EngineOptions,
};
pub use module::{
    ActionOutcome, ActionRequest, HubWindowRow, HubWindowsSlice, HubWindowsStatus, LumaModule,
    ModuleManifest, ModuleState, SearchMode, SearchSink, WarmupContext, WorkbenchMeta,
};
pub use paste::{paste_to_target_app, AX_PASTE_TIMEOUT, NO_PASTE_TARGET_REASON};
pub use port::EnginePort;
pub use ports::{
    looks_secret, AccessibilityError, AccessibilityPort, AppEntry, AppLaunchError, AppSettings,
    AppsCatalogPort, CapabilityPort, ClipboardEntry, ClipboardHistoryRepository,
    ClipboardRepoError, ClockError, ClockPort, DiagnosticsError, DiagnosticsSink,
    FakeAccessibility, FakeCapabilities, FakeKeychain, FakeMarkdownWatcher, FakeOpenPath,
    FakePasteboard, FakePlatformProbe, FakeWindowCatalog, FixedClock, KeychainError, KeychainPort,
    MarkdownWatchPort, MemoryClipboardHistory, MemoryNotesIndex, MemoryQuicklinksRepository,
    MemorySnippetsRepository, NotesDocument, NotesIndexError, NotesIndexRepository, NotesIssue,
    NotesLink, NotesScanReport, NotesScanStatusView, NotesSearchHit, OpenPathError, OpenPathPort,
    PasteboardError, PasteboardPort, PlatformProbePort, QuicklinkEntry, QuicklinksRepoError,
    QuicklinksRepository, SecretLabel, SettingsError, SettingsRepository, SnippetEntry,
    SnippetsRepoError, SnippetsRepository, StorageProbePort, SystemClock, WindowCatalogPort,
    WindowEntry, WindowError,
};
pub use registry::{ModuleRegistry, RegistryError};
