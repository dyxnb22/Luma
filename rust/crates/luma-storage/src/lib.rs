//! Storage adapters for LumaNext application-support paths.

mod clipboard_store;
mod config;
mod importer;
mod migration_ledger;
mod paths;
mod quicklinks_store;
mod snippets_store;

pub use clipboard_store::{looks_secret, ClipboardRow, ClipboardStore, ClipboardStoreError};
pub use config::{ConfigError, ConfigStore, LumaSettings};
pub use importer::{
    dry_run_legacy_dir, import_clipboard_fixture, import_clipboard_fixture_with_ledger,
    import_notes_config_fixture, import_notes_config_fixture_with_ledger, ImportError,
    ImportReport, MigrationLedgerEntry,
};
pub use migration_ledger::{
    get_migration, list_migrations, record_dry_run, rollback_migration, LedgerError,
    MigrationCommitGuard, MigrationKind, MigrationStatus, PersistedMigration,
};
pub use paths::{ensure_luma_next_dirs, luma_next_logs_dir, luma_next_support_dir, PathsError};
pub use quicklinks_store::{QuicklinkRow, QuicklinksStore, QuicklinksStoreError};
pub use snippets_store::{SnippetRow, SnippetsStore, SnippetsStoreError};
