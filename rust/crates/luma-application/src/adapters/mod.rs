//! Storage/platform adapters living next to ports (composition root may also wrap).

mod clipboard_repo;
mod command_recipes_repo;
mod notes_repo;
mod quicklinks_repo;
mod records_repo;
mod settings_repo;
mod snippets_repo;
mod wordbook_repo;

pub use clipboard_repo::SqliteClipboardHistory;
pub use command_recipes_repo::{MemoryCommandRecipesRepository, SqliteCommandRecipesRepository};
pub use notes_repo::SqliteNotesIndex;
pub use quicklinks_repo::SqliteQuicklinksRepository;
pub use records_repo::SqliteRecordsRepository;
pub use settings_repo::TomlSettingsRepository;
pub use snippets_repo::SqliteSnippetsRepository;
pub use wordbook_repo::SqliteWordbookRepository;
