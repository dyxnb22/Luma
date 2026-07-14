//! Injected infrastructure ports. Adapters live in platform/storage; modules only see these.

mod accessibility;
mod apps;
mod clipboard_repo;
mod clock;
mod diagnostics;
mod keychain;
mod markdown_watch;
mod memory_repos;
mod notes_repo;
mod open_path;
mod pasteboard;
mod quicklinks_repo;
mod settings;
mod snippets_repo;
mod window;

pub use accessibility::{AccessibilityError, AccessibilityPort, FakeAccessibility};
pub use apps::{AppEntry, AppLaunchError, AppsCatalogPort};
pub use clipboard_repo::{
    looks_secret, ClipboardEntry, ClipboardHistoryRepository, ClipboardRepoError,
};
pub use clock::{ClockError, ClockPort, FixedClock, SystemClock};
pub use diagnostics::{DiagnosticsError, DiagnosticsSink};
pub use keychain::{FakeKeychain, KeychainError, KeychainPort, SecretLabel};
pub use markdown_watch::MarkdownWatchPort;
pub use memory_repos::{
    MemoryClipboardHistory, MemoryNotesIndex, MemoryQuicklinksRepository, MemorySnippetsRepository,
};
pub use notes_repo::{
    NotesDocument, NotesIndexError, NotesIndexRepository, NotesIssue, NotesLink, NotesScanReport,
    NotesScanStatusView, NotesSearchHit,
};
pub use open_path::{FakeOpenPath, OpenPathError, OpenPathPort};
pub use pasteboard::{FakePasteboard, PasteboardError, PasteboardPort};
pub use quicklinks_repo::{QuicklinkEntry, QuicklinksRepoError, QuicklinksRepository};
pub use settings::{AppSettings, SettingsError, SettingsRepository};
pub use snippets_repo::{SnippetEntry, SnippetsRepoError, SnippetsRepository};
pub use window::{FakeWindowCatalog, WindowCatalogPort, WindowEntry, WindowError};
