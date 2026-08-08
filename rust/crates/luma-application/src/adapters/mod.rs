//! Storage/platform adapters living next to ports (composition root may also wrap).

mod clipboard_repo;
mod command_recipes_repo;
mod recall_repo;
mod records_repo;
mod settings_repo;
mod wordbook_repo;

pub use clipboard_repo::SqliteClipboardHistory;
pub use command_recipes_repo::{MemoryCommandRecipesRepository, SqliteCommandRecipesRepository};
#[allow(unused_imports)]
pub use recall_repo::SqliteRecallRepository;
pub use records_repo::SqliteRecordsRepository;
pub use settings_repo::TomlSettingsRepository;
pub use wordbook_repo::SqliteWordbookRepository;
