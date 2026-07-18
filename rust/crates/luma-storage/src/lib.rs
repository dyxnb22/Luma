//! Storage adapters for LumaNext application-support paths.

mod clipboard_store;
mod command_recipes_builtin;
mod command_recipes_config;
mod command_recipes_meta;
mod config;
mod importer;
mod migration_ledger;
mod notes_discover;
mod notes_ignore;
mod notes_index_store;
mod notes_parse;
mod notes_scan;
mod paths;
mod quicklinks_store;
mod records_parse;
mod records_store;
mod snippets_store;
mod sqlite;
mod ssh_config_parse;
mod ssh_meta_store;
mod timers_store;
mod wordbook_store;

pub use clipboard_store::{looks_secret, ClipboardRow, ClipboardStore, ClipboardStoreError};
pub use command_recipes_builtin::builtin_recipes;
pub use command_recipes_config::{
    command_recipes_config_path, load_recipe_catalog, CommandRecipesConfigError,
};
pub use command_recipes_meta::{CommandRecipesMetaError, CommandRecipesMetaStore};
pub use config::{
    validate_import_project_path, ConfigError, ConfigReadError, ConfigStore, ImportedProject,
    LumaSettings,
};
pub use importer::{
    dry_run_legacy_dir, import_clipboard_fixture, import_clipboard_fixture_with_ledger,
    import_notes_config_fixture, import_notes_config_fixture_with_ledger, ImportError,
    ImportReport, MigrationLedgerEntry,
};
pub use migration_ledger::{
    get_migration, list_migrations, record_dry_run, rollback_migration, LedgerError,
    MigrationCommitGuard, MigrationKind, MigrationStatus, PersistedMigration,
};
pub use notes_discover::{discover, DiscoverResult, SkipReason, SkippedEntry};
pub use notes_ignore::{path_match, IgnoreMatcher, DEFAULT_IGNORE_DIRS};
pub use notes_index_store::{
    contains_cjk, escape_fts_query, DocumentLinkRow, DocumentRow, NotesIndexStore,
    NotesIndexStoreError, ScanIssueRow, SearchHit, ISSUE_FRONTMATTER_WARNING, ISSUE_OVERSIZED,
    ISSUE_SYMLINK_SKIPPED, ISSUE_UNREADABLE, ISSUE_WALK_ERROR,
};
pub use notes_parse::{
    extract_body, extract_links, extract_tags, extract_title, split_frontmatter, ExtractedLink,
    FrontmatterResult, LinkKind, TagsResult, TitleResult,
};
pub use notes_scan::{
    NotesScanError, NotesScanOptions, NotesScanner, ScanMode, ScanReport, ScanStatus,
    DEFAULT_MAX_FILE_BYTES,
};
pub use paths::{ensure_luma_next_dirs, luma_next_logs_dir, luma_next_support_dir, PathsError};
pub use quicklinks_store::{QuicklinkRow, QuicklinksStore, QuicklinksStoreError};
pub use records_store::{
    import_records_with_ledger, now_iso as records_now_iso, preview_import_from_dir,
    RecordCategoryRow, RecordImportApplyReport, RecordImportPreview, RecordRow,
    RecordsImportLedgerReport, RecordsStats, RecordsStore, RecordsStoreError,
};
pub use snippets_store::{SnippetRow, SnippetsStore, SnippetsStoreError};
pub use ssh_config_parse::{
    collect_aliases_from_file, host_alias_is_unsafe, host_pattern_is_wildcard, parse_host_aliases,
    parse_include_paths, resolve_include_path,
};
pub use ssh_meta_store::{SshHostMetaRow, SshMetaStore, SshMetaStoreError};
pub use timers_store::{TimerRow, TimersStore, TimersStoreError};
pub use wordbook_store::{
    now_iso, schedule_review, ImportContentReport, WordContent, WordImportRow, WordRow,
    WordbookReadOnlyError, WordbookReadOnlyStore, WordbookStats, WordbookStore, WordbookStoreError,
    WordpetImportReport,
};
