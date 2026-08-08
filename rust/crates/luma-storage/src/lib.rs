//! Storage adapters for LumaNext application-support paths.

mod clipboard_store;
mod command_recipes_builtin;
mod command_recipes_config;
mod command_recipes_meta;
mod config;
mod importer;
mod migration_ledger;
mod paths;
mod recall_store;
mod records_parse;
mod records_store;
mod sqlite;
mod wordbook_store;

pub use clipboard_store::{looks_secret, ClipboardRow, ClipboardStore, ClipboardStoreError};
pub use command_recipes_builtin::builtin_recipes;
pub use command_recipes_config::{
    command_recipes_config_path, load_recipe_catalog, CommandRecipesConfigError,
};
pub use command_recipes_meta::{CommandRecipesMetaError, CommandRecipesMetaStore};
pub use config::{
    validate_import_project_path, ConfigError, ConfigStore, ImportedProject, LumaSettings,
};
pub use importer::{
    dry_run_legacy_dir, import_clipboard_fixture, import_clipboard_fixture_with_ledger,
    ImportError, ImportReport, MigrationLedgerEntry,
};
pub use migration_ledger::{
    get_migration, list_migrations, record_dry_run, rollback_migration, LedgerError,
    MigrationCommitGuard, MigrationKind, MigrationStatus, PersistedMigration,
};
pub use paths::{ensure_luma_next_dirs, luma_next_logs_dir, luma_next_support_dir, PathsError};
pub use recall_store::{RecallRow, RecallStore, RecallStoreError, MAX_RECALL_OBJECTS};
pub use records_store::{
    import_records_with_ledger, now_iso as records_now_iso, preview_import_from_dir,
    RecordCategoryRow, RecordImportApplyReport, RecordImportPreview, RecordRow,
    RecordsImportLedgerReport, RecordsStats, RecordsStore, RecordsStoreError,
};
pub use wordbook_store::{
    now_iso, schedule_review, ImportContentReport, WordContent, WordImportRow, WordRow,
    WordbookStats, WordbookStore, WordbookStoreError, WordpetImportReport,
};
