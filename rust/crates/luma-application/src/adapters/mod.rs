//! Storage/platform adapters living next to ports (composition root may also wrap).

mod clipboard_repo;
mod command_recipes_repo;
mod database_portals_repo;
mod quicklinks_repo;
mod recall_repo;
mod records_repo;
mod renewals_repo;
mod settings_repo;
mod snippets_repo;
mod ssh_meta_repo;
mod timers_repo;
mod wordbook_repo;

pub use clipboard_repo::SqliteClipboardHistory;
pub use command_recipes_repo::{MemoryCommandRecipesRepository, SqliteCommandRecipesRepository};
pub use database_portals_repo::SqliteDatabasePortalsRepository;
pub use quicklinks_repo::SqliteQuicklinksRepository;
#[allow(unused_imports)]
pub use recall_repo::SqliteRecallRepository;
pub use records_repo::SqliteRecordsRepository;
pub use renewals_repo::SqliteRenewalsRepository;
pub use settings_repo::TomlSettingsRepository;
pub use snippets_repo::SqliteSnippetsRepository;
pub use ssh_meta_repo::SqliteSshMetaRepository;
pub use timers_repo::SqliteTimersRepository;
pub use wordbook_repo::SqliteWordbookRepository;
