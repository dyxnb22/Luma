//! Storage adapters for LumaNext application-support paths.

mod clipboard_store;
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
mod snippets_store;
mod sqlite;
mod wordbook_store;

pub use clipboard_store::{looks_secret, ClipboardRow, ClipboardStore, ClipboardStoreError};
pub use config::{
    validate_import_project_path, ConfigError, ConfigStore, ImportedProject, LumaSettings,
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
pub use paths::{
    ensure_luma_next_dirs, luma_next_diagnostics_dir, luma_next_logs_dir, luma_next_support_dir,
    PathsError,
};
pub use quicklinks_store::{QuicklinkRow, QuicklinksStore, QuicklinksStoreError};
pub use snippets_store::{SnippetRow, SnippetsStore, SnippetsStoreError};
pub use wordbook_store::{
    now_iso, schedule_review, ImportContentReport, WordContent, WordImportRow, WordRow,
    WordbookStats, WordbookStore, WordbookStoreError, WordpetImportReport,
};
