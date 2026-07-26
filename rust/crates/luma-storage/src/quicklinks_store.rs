use crate::paths::{ensure_luma_next_dirs, luma_next_support_dir, PathsError};
use chrono::Utc;
use luma_domain::MAX_QUICKLINKS;
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Max UTF-8 bytes for a quicklink URL on upsert (personal-use guardrail).
pub use luma_domain::MAX_QUICKLINK_URL_BYTES as MAX_URL_BYTES;

/// Max UTF-8 bytes for a quicklink trigger on upsert.
pub use luma_domain::MAX_QUICKLINK_TRIGGER_BYTES as MAX_TRIGGER_BYTES;

#[derive(Debug, Error)]
pub enum QuicklinksStoreError {
    #[error(transparent)]
    Paths(#[from] PathsError),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Msg(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuicklinkRow {
    pub trigger: String,
    pub url: String,
}

pub struct QuicklinksStore {
    path: PathBuf,
}

impl QuicklinksStore {
    pub fn luma_next_default() -> Result<Self, QuicklinksStoreError> {
        ensure_luma_next_dirs()?;
        Self::with_path(luma_next_support_dir()?.join("quicklinks.sqlite"))
    }

    pub fn with_path(path: PathBuf) -> Result<Self, QuicklinksStoreError> {
        let store = Self { path };
        store.init()?;
        Ok(store)
    }

    fn connect(&self) -> Result<Connection, QuicklinksStoreError> {
        crate::sqlite::open_connection(&self.path).map_err(Into::into)
    }

    fn init(&self) -> Result<(), QuicklinksStoreError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        self.connect()?.execute_batch(
            "CREATE TABLE IF NOT EXISTS quicklinks (trigger TEXT PRIMARY KEY, url TEXT NOT NULL);",
        )?;
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<QuicklinkRow>, QuicklinksStoreError> {
        let conn = self.connect()?;
        let mut statement =
            conn.prepare("SELECT trigger, url FROM quicklinks ORDER BY trigger LIMIT ?1")?;
        let rows: Vec<QuicklinkRow> = statement
            .query_map(params![(MAX_QUICKLINKS + 1) as i64], |row| {
                Ok(QuicklinkRow {
                    trigger: row.get(0)?,
                    url: row.get(1)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        // Legacy/manual edits can exceed the write cap. Keep the extra row
        // visible so the module can delete it and recover; only `upsert`
        // enforces the capacity for new entries.
        Ok(rows)
    }

    pub fn upsert(&self, trigger: &str, url: &str) -> Result<(), QuicklinksStoreError> {
        if trigger.len() > MAX_TRIGGER_BYTES {
            return Err(QuicklinksStoreError::Msg(format!(
                "quicklink trigger exceeds max size ({MAX_TRIGGER_BYTES} bytes)"
            )));
        }
        if url.len() > MAX_URL_BYTES {
            return Err(QuicklinksStoreError::Msg(format!(
                "quicklink url exceeds max size ({MAX_URL_BYTES} bytes)"
            )));
        }
        let conn = self.connect()?;
        let tx = conn.unchecked_transaction()?;
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM quicklinks WHERE trigger = ?1)",
            params![trigger],
            |row| row.get(0),
        )?;
        if !exists {
            let count: i64 =
                tx.query_row("SELECT COUNT(*) FROM quicklinks", [], |row| row.get(0))?;
            if count >= MAX_QUICKLINKS as i64 {
                return Err(QuicklinksStoreError::Msg(format!(
                    "quicklinks capacity reached ({MAX_QUICKLINKS}); delete an entry before adding another"
                )));
            }
        }
        tx.execute(
            "INSERT INTO quicklinks (trigger, url) VALUES (?1, ?2)
             ON CONFLICT(trigger) DO UPDATE SET url = excluded.url",
            params![trigger, url],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn delete(&self, trigger: &str) -> Result<(), QuicklinksStoreError> {
        self.connect()?.execute(
            "DELETE FROM quicklinks WHERE trigger = ?1",
            params![trigger],
        )?;
        Ok(())
    }

    pub fn backup(&self) -> Result<PathBuf, QuicklinksStoreError> {
        ensure_luma_next_dirs()?;
        let backups = luma_next_support_dir()?.join("backups");
        std::fs::create_dir_all(&backups)?;
        let now = Utc::now();
        let stamp = format!(
            "{}-{:03}",
            now.format("%Y%m%d-%H%M%S"),
            now.timestamp_subsec_millis()
        );
        let dest = backups.join(format!("quicklinks-backup-{stamp}.sqlite"));
        let tmp = backups.join(format!("quicklinks-backup-{stamp}.sqlite.tmp"));
        let _ = std::fs::remove_file(&tmp);
        let quoted = sqlite_path_literal(&tmp)?;
        self.connect()?
            .execute_batch(&format!("VACUUM INTO {quoted}"))?;
        std::fs::rename(&tmp, &dest)?;
        Ok(dest)
    }
}

fn sqlite_path_literal(path: &Path) -> Result<String, QuicklinksStoreError> {
    let text = path
        .to_str()
        .ok_or_else(|| QuicklinksStoreError::Msg("backup path is not valid UTF-8".into()))?;
    if text.contains('\0') {
        return Err(QuicklinksStoreError::Msg(
            "backup path contains a NUL byte".into(),
        ));
    }
    Ok(format!("'{}'", text.replace('\'', "''")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn upsert_list_delete() {
        let dir = tempdir().unwrap();
        let store = QuicklinksStore::with_path(dir.path().join("ql.sqlite")).unwrap();
        store.upsert("gh", "https://github.com/{query}").unwrap();
        assert_eq!(store.list().unwrap().len(), 1);
        store.delete("gh").unwrap();
        assert!(store.list().unwrap().is_empty());
    }

    #[test]
    fn backup_writes_consistent_snapshot_under_lumanext() {
        let dir = tempdir().unwrap();
        let _env = crate::paths::LumaNextTestEnvGuard::override_paths(
            dir.path(),
            &dir.path().join("logs"),
        );
        let store = QuicklinksStore::luma_next_default().unwrap();
        store.upsert("gh", "https://github.com").unwrap();

        let backup = store.backup().unwrap();
        let snapshot = QuicklinksStore::with_path(backup.clone()).unwrap();
        assert_eq!(snapshot.list().unwrap()[0].trigger, "gh");
        assert!(backup.to_string_lossy().contains("quicklinks-backup-"));
    }

    #[test]
    fn upsert_rejects_oversized_url() {
        let dir = tempdir().unwrap();
        let store = QuicklinksStore::with_path(dir.path().join("ql.sqlite")).unwrap();
        let huge = "x".repeat(MAX_URL_BYTES + 1);
        let err = store.upsert("big", &huge).unwrap_err().to_string();
        assert!(err.contains("max size"), "{err}");
        assert!(store.list().unwrap().is_empty());
    }

    #[test]
    fn capacity_rejects_new_rows_but_allows_updates() {
        let dir = tempdir().unwrap();
        let store = QuicklinksStore::with_path(dir.path().join("ql.sqlite")).unwrap();
        let conn = store.connect().unwrap();
        let tx = conn.unchecked_transaction().unwrap();
        for i in 0..MAX_QUICKLINKS {
            tx.execute(
                "INSERT INTO quicklinks (trigger, url) VALUES (?1, 'https://example.com')",
                params![format!("q{i:04}")],
            )
            .unwrap();
        }
        tx.commit().unwrap();

        store.upsert("q0000", "https://updated.example").unwrap();
        let err = store
            .upsert("overflow", "https://example.com")
            .unwrap_err()
            .to_string();
        assert!(err.contains("capacity reached"), "{err}");
        assert_eq!(store.list().unwrap().len(), MAX_QUICKLINKS);
    }

    #[test]
    fn over_capacity_rows_remain_listable_for_recovery() {
        let dir = tempdir().unwrap();
        let store = QuicklinksStore::with_path(dir.path().join("ql.sqlite")).unwrap();
        let conn = store.connect().unwrap();
        for i in 0..=MAX_QUICKLINKS {
            conn.execute(
                "INSERT INTO quicklinks (trigger, url) VALUES (?1, 'https://example.com')",
                params![format!("q{i:04}")],
            )
            .unwrap();
        }
        assert_eq!(store.list().unwrap().len(), MAX_QUICKLINKS + 1);
        store.delete("q0000").unwrap();
        assert_eq!(store.list().unwrap().len(), MAX_QUICKLINKS);
    }
}
