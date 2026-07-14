//! Storage/platform adapters living next to ports (composition root may also wrap).

mod clipboard_repo;
mod diagnostics;
mod notes_repo;
mod quicklinks_repo;
mod settings_repo;
mod snippets_repo;

pub use clipboard_repo::SqliteClipboardHistory;
pub use diagnostics::FsDiagnosticsSink;
pub use notes_repo::SqliteNotesIndex;
pub use quicklinks_repo::SqliteQuicklinksRepository;
pub use settings_repo::TomlSettingsRepository;
pub use snippets_repo::SqliteSnippetsRepository;
