//! Read-only legacy / fixture importers. Never modify source files.

use crate::clipboard_store::{looks_secret, ClipboardStore, ClipboardStoreError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MigrationLedgerEntry {
    pub source_fingerprint: String,
    pub schema_version: u32,
    pub imported: u64,
    pub skipped: u64,
    pub errors: u64,
    pub dry_run: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub migration_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImportReport {
    pub ledger: MigrationLedgerEntry,
    pub notes: Vec<String>,
}

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Store(#[from] ClipboardStoreError),
}

/// Dry-run only: inspect a path without reading user secrets into logs.
pub fn dry_run_legacy_dir(legacy_support: PathBuf) -> ImportReport {
    let exists = legacy_support.exists();
    let mut notes = vec![if exists {
        "legacy directory exists (not modified)".into()
    } else {
        "legacy directory missing".into()
    }];
    let clip = legacy_support.join("clipboard-history.json");
    if clip.exists() {
        notes.push(format!(
            "found clipboard-history.json ({} bytes) — use `luma migrate clipboard --legacy <path> --commit` (source read-only)",
            fs::metadata(&clip).map(|m| m.len()).unwrap_or(0)
        ));
    }
    ImportReport {
        ledger: MigrationLedgerEntry {
            source_fingerprint: format!("path:{}", legacy_support.display()),
            schema_version: 0,
            imported: 0,
            skipped: 0,
            errors: 0,
            dry_run: true,
            migration_id: None,
        },
        notes,
    }
}

#[derive(Clone, Debug, Deserialize)]
struct LegacyClipboardEntry {
    text: String,
    #[serde(default)]
    pinned: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum LegacyClipboardFile {
    Array(Vec<LegacyClipboardEntry>),
    Wrapped { entries: Vec<LegacyClipboardEntry> },
}

impl LegacyClipboardFile {
    fn entries(self) -> Vec<LegacyClipboardEntry> {
        match self {
            Self::Array(v) => v,
            Self::Wrapped { entries } => entries,
        }
    }
}

/// Import desensitized clipboard fixture JSON into a LumaNext store.
/// Source file is read-only. Secrets are skipped. Idempotent by text dedup of newest match.
/// Does not persist a migration ledger — use [`import_clipboard_fixture_with_ledger`] from CLI.
pub fn import_clipboard_fixture(
    fixture: &Path,
    store: &ClipboardStore,
    dry_run: bool,
) -> Result<ImportReport, ImportError> {
    import_clipboard_fixture_with_ledger(fixture, store, dry_run, false)
}

pub fn import_clipboard_fixture_with_ledger(
    fixture: &Path,
    store: &ClipboardStore,
    dry_run: bool,
    persist_ledger: bool,
) -> Result<ImportReport, ImportError> {
    use crate::migration_ledger::{record_dry_run, MigrationCommitGuard, MigrationKind};
    use crate::paths::luma_next_support_dir;

    let guard = if persist_ledger && !dry_run {
        let settings = luma_next_support_dir()
            .map(|p| p.join("settings.toml"))
            .unwrap_or_default();
        Some(
            MigrationCommitGuard::begin(
                MigrationKind::Clipboard,
                fixture,
                &[
                    (store.path(), "clipboard.sqlite"),
                    (&settings, "settings.toml"),
                ],
            )
            .map_err(|e| ImportError::Io(std::io::Error::other(e.to_string())))?,
        )
    } else {
        None
    };

    let raw = fs::read_to_string(fixture)?;
    let file: LegacyClipboardFile = serde_json::from_str(&raw)?;
    let entries = file.entries();
    let mut imported = 0u64;
    let mut skipped = 0u64;
    let mut errors = 0u64;
    let mut notes = Vec::new();

    for entry in entries {
        if looks_secret(&entry.text) {
            skipped += 1;
            continue;
        }
        if dry_run {
            imported += 1;
            continue;
        }
        match store.search(&entry.text.chars().take(32).collect::<String>(), 5) {
            Ok(hits) if hits.iter().any(|h| h.text == entry.text) => {
                skipped += 1;
            }
            Ok(_) => match store.insert(&entry.text, entry.pinned) {
                Ok(_) => imported += 1,
                Err(_) => errors += 1,
            },
            Err(_) => errors += 1,
        }
    }

    notes.push(format!("source={}", fixture.display()));
    if dry_run {
        notes.push("dry_run=true; store not written".into());
    }

    let migration_id = if persist_ledger {
        if dry_run {
            let rec = record_dry_run(
                MigrationKind::Clipboard,
                fixture,
                imported,
                skipped,
                errors,
                notes.clone(),
            )
            .map_err(|e| ImportError::Io(std::io::Error::other(e.to_string())))?;
            Some(rec.migration_id)
        } else if let Some(g) = guard {
            let rec = g
                .finalize(
                    MigrationKind::Clipboard,
                    fixture,
                    imported,
                    skipped,
                    errors,
                    notes.clone(),
                )
                .map_err(|e| ImportError::Io(std::io::Error::other(e.to_string())))?;
            Some(rec.migration_id)
        } else {
            None
        }
    } else {
        None
    };

    Ok(ImportReport {
        ledger: MigrationLedgerEntry {
            source_fingerprint: format!("file:{}", fixture.display()),
            schema_version: 1,
            imported,
            skipped,
            errors,
            dry_run,
            migration_id,
        },
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clipboard_store::ClipboardStore;
    use tempfile::tempdir;

    #[test]
    fn dry_run_does_not_create_files() {
        let dir = tempdir().unwrap();
        let report = dry_run_legacy_dir(dir.path().to_path_buf());
        assert!(report.ledger.dry_run);
        assert_eq!(report.ledger.imported, 0);
    }

    #[test]
    fn fixture_import_skips_secrets_and_is_idempotent() {
        let dir = tempdir().unwrap();
        let fixture = dir.path().join("clip.json");
        fs::write(
            &fixture,
            r#"[{"text":"invoice 99"},{"text":"password=secret"},{"text":"invoice 99"}]"#,
        )
        .unwrap();
        let store = ClipboardStore::with_path(dir.path().join("out.sqlite")).unwrap();
        let r1 = import_clipboard_fixture(&fixture, &store, false).unwrap();
        assert_eq!(r1.ledger.imported, 1);
        assert!(r1.ledger.skipped >= 1);
        let r2 = import_clipboard_fixture(&fixture, &store, false).unwrap();
        assert_eq!(r2.ledger.imported, 0);
        assert_eq!(store.count().unwrap(), 1);
    }

    #[test]
    fn dry_run_fixture_does_not_write() {
        let dir = tempdir().unwrap();
        let fixture = dir.path().join("clip.json");
        fs::write(&fixture, r#"[{"text":"hello"}]"#).unwrap();
        let store = ClipboardStore::with_path(dir.path().join("out.sqlite")).unwrap();
        let r = import_clipboard_fixture(&fixture, &store, true).unwrap();
        assert!(r.ledger.dry_run);
        assert_eq!(r.ledger.imported, 1);
        assert_eq!(store.count().unwrap(), 0);
    }

    #[test]
    fn commit_ledger_and_rollback_clipboard() {
        let dir = tempdir().unwrap();
        let support = dir.path().join("LumaNext");
        fs::create_dir_all(&support).unwrap();
        let _env =
            crate::paths::LumaNextTestEnvGuard::override_paths(&support, &dir.path().join("logs"));

        let fixture = dir.path().join("clip.json");
        fs::write(&fixture, r#"[{"text":"fixture-alpha"}]"#).unwrap();
        let store = ClipboardStore::with_path(support.join("clipboard.sqlite")).unwrap();
        assert_eq!(store.count().unwrap(), 0);

        let report = import_clipboard_fixture_with_ledger(&fixture, &store, false, true).unwrap();
        assert!(!report.ledger.dry_run);
        let mig = report.ledger.migration_id.clone().expect("migration id");
        assert_eq!(store.count().unwrap(), 1);

        let rolled = crate::migration_ledger::rollback_migration(
            &mig,
            &[("clipboard.sqlite", store.path())],
        )
        .unwrap();
        assert!(matches!(
            rolled.status,
            crate::migration_ledger::MigrationStatus::RolledBack
        ));
        // Re-open store after file restore
        let store2 = ClipboardStore::with_path(support.join("clipboard.sqlite")).unwrap();
        assert_eq!(store2.count().unwrap(), 0);
    }
}
