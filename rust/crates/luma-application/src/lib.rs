//! Application engine. No ratatui/crossterm.

mod adapters;
mod engine;
mod module;
mod port;
mod ports;
mod registry;

pub use adapters::{
    FsDiagnosticsSink, SqliteClipboardHistory, SqliteQuicklinksRepository,
    SqliteSnippetsRepository, TomlSettingsRepository,
};
pub use engine::{list_modules_json, run_action, run_doctor, run_query, Engine, EngineOptions};
pub use module::{
    ActionOutcome, ActionRequest, LumaModule, ModuleManifest, ModuleState, SearchMode, SearchSink,
    WarmupContext,
};
pub use port::EnginePort;
pub use ports::{
    looks_secret, AccessibilityError, AccessibilityPort, AppEntry, AppLaunchError, AppSettings,
    AppsCatalogPort, ClipboardEntry, ClipboardHistoryRepository, ClipboardRepoError, ClockError,
    ClockPort, DiagnosticsError, DiagnosticsSink, EventKitError, EventKitPort, FakeAccessibility,
    FakeKeychain, FakeOpenPath, FixedClock, KeychainError, KeychainPort, MarkdownWatchPort,
    MemoryClipboardHistory, MemoryQuicklinksRepository, MemorySnippetsRepository, OpenPathError,
    OpenPathPort, PasteboardError, PasteboardPort, ProcessCatalogPort, ProcessEntry, ProcessError,
    QuicklinkEntry, QuicklinksRepoError, QuicklinksRepository, ReminderItem, RemindersAuth,
    SecretLabel, SettingsError, SettingsRepository, SnippetEntry, SnippetsRepoError,
    SnippetsRepository, SystemClock, TranslationError, TranslationResult, TranslatorPort,
};
pub use registry::{ModuleRegistry, RegistryError};
