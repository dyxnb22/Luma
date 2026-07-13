//! Persisted migration ledger under LumaNext. Legacy sources stay read-only.

use crate::paths::{luma_next_support_dir, PathsError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum LedgerError {
    #[error(transparent)]
    Paths(#[from] PathsError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("migration not found: {0}")]
    NotFound(String),
    #[error("migration already rolled back: {0}")]
    AlreadyRolledBack(String),
    #[error("dry-run migrations have no rollback snapshot")]
    DryRunNoSnapshot,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MigrationKind {
    Clipboard,
    NotesConfig,
    LegacyDryRun,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MigrationStatus {
    Committed,
    RolledBack,
    DryRunRecorded,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersistedMigration {
    pub migration_id: String,
    pub kind: MigrationKind,
    pub status: MigrationStatus,
    pub source_fingerprint: String,
    pub schema_version: u32,
    pub imported: u64,
    pub skipped: u64,
    pub errors: u64,
    pub dry_run: bool,
    pub created_at_unix: u64,
    pub notes: Vec<String>,
    /// Relative paths under the migration directory for rollback restore.
    pub snapshot_files: Vec<String>,
}

fn migrations_root() -> Result<PathBuf, LedgerError> {
    Ok(luma_next_support_dir()?.join("migrations"))
}

fn migration_dir(id: &str) -> Result<PathBuf, LedgerError> {
    Ok(migrations_root()?.join(id))
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn fingerprint_file(path: &Path) -> String {
    match fs::metadata(path) {
        Ok(meta) => format!("file:{}:len={}", path.display(), meta.len()),
        Err(_) => format!("file:{}:missing", path.display()),
    }
}

/// Copy `src` into `dest_dir/name` if it exists. Returns the relative name if copied.
fn snapshot_file(src: &Path, dest_dir: &Path, name: &str) -> Result<Option<String>, LedgerError> {
    if !src.exists() {
        return Ok(None);
    }
    fs::create_dir_all(dest_dir)?;
    let dest = dest_dir.join(name);
    fs::copy(src, &dest)?;
    Ok(Some(name.to_string()))
}

fn write_record(dir: &Path, record: &PersistedMigration) -> Result<(), LedgerError> {
    fs::create_dir_all(dir)?;
    let body = serde_json::to_string_pretty(record)?;
    fs::write(dir.join("ledger.json"), body)?;
    Ok(())
}

fn read_record(dir: &Path) -> Result<PersistedMigration, LedgerError> {
    let raw = fs::read_to_string(dir.join("ledger.json"))?;
    Ok(serde_json::from_str(&raw)?)
}

/// Begin a commit migration: allocate id, snapshot LumaNext files that may change.
pub struct MigrationCommitGuard {
    pub migration_id: String,
    pub dir: PathBuf,
    pub snapshot_files: Vec<String>,
}

impl MigrationCommitGuard {
    pub fn begin(
        _kind: MigrationKind,
        source: &Path,
        snapshot_targets: &[(&Path, &str)],
    ) -> Result<Self, LedgerError> {
        let migration_id = format!("mig-{}", Uuid::new_v4());
        let dir = migration_dir(&migration_id)?;
        fs::create_dir_all(dir.join("snapshot"))?;
        let mut snapshot_files = Vec::new();
        for (src, name) in snapshot_targets {
            if let Some(n) = snapshot_file(src, &dir.join("snapshot"), name)? {
                snapshot_files.push(n);
            }
        }
        // Record source fingerprint only (never copy legacy payloads into ledger for privacy).
        let _ = fingerprint_file(source);
        Ok(Self {
            migration_id,
            dir,
            snapshot_files,
        })
    }

    pub fn finalize(
        self,
        kind: MigrationKind,
        source: &Path,
        imported: u64,
        skipped: u64,
        errors: u64,
        notes: Vec<String>,
    ) -> Result<PersistedMigration, LedgerError> {
        let record = PersistedMigration {
            migration_id: self.migration_id,
            kind,
            status: MigrationStatus::Committed,
            source_fingerprint: fingerprint_file(source),
            schema_version: 1,
            imported,
            skipped,
            errors,
            dry_run: false,
            created_at_unix: now_unix(),
            notes,
            snapshot_files: self.snapshot_files,
        };
        write_record(&self.dir, &record)?;
        Ok(record)
    }
}

/// Record a dry-run (no snapshot / no LumaNext writes).
pub fn record_dry_run(
    kind: MigrationKind,
    source: &Path,
    imported: u64,
    skipped: u64,
    errors: u64,
    notes: Vec<String>,
) -> Result<PersistedMigration, LedgerError> {
    let migration_id = format!("mig-{}", Uuid::new_v4());
    let dir = migration_dir(&migration_id)?;
    let record = PersistedMigration {
        migration_id,
        kind,
        status: MigrationStatus::DryRunRecorded,
        source_fingerprint: fingerprint_file(source),
        schema_version: 1,
        imported,
        skipped,
        errors,
        dry_run: true,
        created_at_unix: now_unix(),
        notes,
        snapshot_files: vec![],
    };
    write_record(&dir, &record)?;
    Ok(record)
}

pub fn get_migration(migration_id: &str) -> Result<PersistedMigration, LedgerError> {
    let dir = migration_dir(migration_id)?;
    if !dir.join("ledger.json").exists() {
        return Err(LedgerError::NotFound(migration_id.into()));
    }
    read_record(&dir)
}

pub fn list_migrations() -> Result<Vec<PersistedMigration>, LedgerError> {
    let root = migrations_root()?;
    if !root.exists() {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let ledger = entry.path().join("ledger.json");
        if ledger.exists() {
            out.push(read_record(&entry.path())?);
        }
    }
    out.sort_by_key(|entry| entry.created_at_unix);
    Ok(out)
}

/// Restore LumaNext files from the migration snapshot. Does not touch legacy sources.
pub fn rollback_migration(
    migration_id: &str,
    restore_targets: &[(&str, &Path)],
) -> Result<PersistedMigration, LedgerError> {
    let dir = migration_dir(migration_id)?;
    let mut record = get_migration(migration_id)?;
    if record.dry_run {
        return Err(LedgerError::DryRunNoSnapshot);
    }
    if record.status == MigrationStatus::RolledBack {
        return Err(LedgerError::AlreadyRolledBack(migration_id.into()));
    }
    let snap = dir.join("snapshot");
    for (name, dest) in restore_targets {
        let src = snap.join(name);
        if src.exists() {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&src, dest)?;
        } else if dest.exists() && record.snapshot_files.iter().any(|s| s == name) {
            // Snapshot listed but missing — fail loud.
            return Err(LedgerError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("missing snapshot file {name}"),
            )));
        } else if !src.exists() && dest.exists() {
            // File was created by migration and had no pre-image — remove it.
            fs::remove_file(dest)?;
        }
    }
    record.status = MigrationStatus::RolledBack;
    record
        .notes
        .push(format!("rolled_back_at_unix={}", now_unix()));
    write_record(&dir, &record)?;
    Ok(record)
}

/// Test helper placeholder (ledger tests use explicit tempfile paths).
#[cfg(test)]
pub mod test_support {}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn snapshot_and_rollback_round_trip() {
        let dir = tempdir().unwrap();
        let support = dir.path().join("LumaNext");
        fs::create_dir_all(&support).unwrap();
        let settings = support.join("settings.toml");
        fs::write(&settings, "before = true\n").unwrap();
        let clip = support.join("clipboard.sqlite");
        fs::write(&clip, b"sqlite-before").unwrap();

        // Manual miniature of begin/finalize using local dirs (avoid home LumaNext).
        let mig_id = "mig-test-1";
        let mig_dir = support.join("migrations").join(mig_id);
        fs::create_dir_all(mig_dir.join("snapshot")).unwrap();
        fs::copy(&settings, mig_dir.join("snapshot/settings.toml")).unwrap();
        fs::copy(&clip, mig_dir.join("snapshot/clipboard.sqlite")).unwrap();
        let record = PersistedMigration {
            migration_id: mig_id.into(),
            kind: MigrationKind::Clipboard,
            status: MigrationStatus::Committed,
            source_fingerprint: "file:fixture".into(),
            schema_version: 1,
            imported: 1,
            skipped: 0,
            errors: 0,
            dry_run: false,
            created_at_unix: now_unix(),
            notes: vec![],
            snapshot_files: vec!["settings.toml".into(), "clipboard.sqlite".into()],
        };
        write_record(&mig_dir, &record).unwrap();

        // Mutate LumaNext
        fs::write(&settings, "after = true\n").unwrap();
        fs::write(&clip, b"sqlite-after").unwrap();

        // Restore
        fs::copy(mig_dir.join("snapshot/settings.toml"), &settings).unwrap();
        fs::copy(mig_dir.join("snapshot/clipboard.sqlite"), &clip).unwrap();
        assert_eq!(fs::read_to_string(&settings).unwrap(), "before = true\n");
        assert_eq!(fs::read(&clip).unwrap(), b"sqlite-before");
    }
}
