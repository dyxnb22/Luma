use crate::paths::{ensure_luma_next_dirs, luma_next_support_dir, PathsError};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const MAX_RENEWALS_ROWS: usize = 1_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenewalRow {
    pub id: i64,
    pub name: String,
    pub category: String,
    pub amount_minor: Option<i64>,
    pub currency: Option<String>,
    pub cadence_kind: String,
    pub cadence_value: Option<i64>,
    pub anchor_month: Option<u32>,
    pub anchor_day: Option<u32>,
    pub next_due_date: String,
    pub auto_renew: bool,
    pub status: String,
    pub url: Option<String>,
    pub note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Error)]
pub enum RenewalsStoreError {
    #[error(transparent)]
    Paths(#[from] PathsError),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("renewal not found")]
    NotFound,
    #[error("renewal update conflict (stale)")]
    Conflict,
    #[error("renewals capacity reached ({MAX_RENEWALS_ROWS})")]
    Capacity,
    #[error("{0}")]
    Msg(String),
}

pub struct RenewalsStore {
    path: PathBuf,
}

impl RenewalsStore {
    pub fn luma_next_default() -> Result<Self, RenewalsStoreError> {
        ensure_luma_next_dirs()?;
        Self::with_path(luma_next_support_dir()?.join("renewals.sqlite"))
    }

    pub fn with_path(path: PathBuf) -> Result<Self, RenewalsStoreError> {
        let store = Self { path };
        store.init()?;
        Ok(store)
    }

    fn connect(&self) -> Result<Connection, RenewalsStoreError> {
        crate::sqlite::open_connection(&self.path).map_err(Into::into)
    }

    fn init(&self) -> Result<(), RenewalsStoreError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        self.connect()?.execute_batch(
            "CREATE TABLE IF NOT EXISTS renewals (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                category TEXT NOT NULL,
                amount_minor INTEGER,
                currency TEXT,
                cadence_kind TEXT NOT NULL,
                cadence_value INTEGER,
                anchor_month INTEGER,
                anchor_day INTEGER,
                next_due_date TEXT NOT NULL,
                auto_renew INTEGER NOT NULL,
                status TEXT NOT NULL,
                url TEXT,
                note TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_renewals_due
             ON renewals(status, next_due_date, id);",
        )?;
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<RenewalRow>, RenewalsStoreError> {
        let conn = self.connect()?;
        let mut statement = conn.prepare(
            "SELECT id, name, category, amount_minor, currency, cadence_kind, cadence_value,
                    anchor_month, anchor_day, next_due_date, auto_renew, status, url, note,
                    created_at, updated_at
             FROM renewals ORDER BY next_due_date ASC, id ASC LIMIT ?1",
        )?;
        let rows: Result<Vec<_>, rusqlite::Error> = statement
            .query_map(params![(MAX_RENEWALS_ROWS + 1) as i64], row_from_sql)?
            .collect();
        Ok(rows?)
    }

    pub fn get(&self, id: i64) -> Result<Option<RenewalRow>, RenewalsStoreError> {
        self.connect()?
            .query_row(
                "SELECT id, name, category, amount_minor, currency, cadence_kind, cadence_value,
                        anchor_month, anchor_day, next_due_date, auto_renew, status, url, note,
                        created_at, updated_at
                 FROM renewals WHERE id = ?1",
                params![id],
                row_from_sql,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn insert(&self, row: &RenewalRow) -> Result<RenewalRow, RenewalsStoreError> {
        let conn = self.connect()?;
        let tx = conn.unchecked_transaction()?;
        let count: i64 = tx.query_row("SELECT COUNT(*) FROM renewals", [], |row| row.get(0))?;
        if count >= MAX_RENEWALS_ROWS as i64 {
            return Err(RenewalsStoreError::Capacity);
        }
        tx.execute(
            "INSERT INTO renewals (
                name, category, amount_minor, currency, cadence_kind, cadence_value,
                anchor_month, anchor_day, next_due_date, auto_renew, status, url, note,
                created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                row.name,
                row.category,
                row.amount_minor,
                row.currency,
                row.cadence_kind,
                row.cadence_value,
                row.anchor_month,
                row.anchor_day,
                row.next_due_date,
                row.auto_renew as i64,
                row.status,
                row.url,
                row.note,
                row.created_at,
                row.updated_at,
            ],
        )?;
        let id = tx.last_insert_rowid();
        tx.commit()?;
        let mut stored = row.clone();
        stored.id = id;
        Ok(stored)
    }

    pub fn update(
        &self,
        row: &RenewalRow,
        expected_updated_at: &str,
    ) -> Result<(), RenewalsStoreError> {
        let changed = self.connect()?.execute(
            "UPDATE renewals SET name=?2, category=?3, amount_minor=?4, currency=?5,
                cadence_kind=?6, cadence_value=?7, anchor_month=?8, anchor_day=?9,
                next_due_date=?10, auto_renew=?11, status=?12, url=?13, note=?14,
                updated_at=?15 WHERE id=?1 AND updated_at=?16",
            params![
                row.id,
                row.name,
                row.category,
                row.amount_minor,
                row.currency,
                row.cadence_kind,
                row.cadence_value,
                row.anchor_month,
                row.anchor_day,
                row.next_due_date,
                row.auto_renew as i64,
                row.status,
                row.url,
                row.note,
                row.updated_at,
                expected_updated_at,
            ],
        )?;
        if changed == 0 {
            return Err(if self.get(row.id)?.is_some() {
                RenewalsStoreError::Conflict
            } else {
                RenewalsStoreError::NotFound
            });
        }
        Ok(())
    }

    pub fn mark_paid(
        &self,
        id: i64,
        expected_due_date: &str,
        expected_updated_at: &str,
        next_due_date: &str,
        status: &str,
        updated_at: &str,
    ) -> Result<(), RenewalsStoreError> {
        let conn = self.connect()?;
        let tx = conn.unchecked_transaction()?;
        let changed = tx.execute(
            "UPDATE renewals SET next_due_date=?2, status=?3, updated_at=?4
             WHERE id=?1 AND next_due_date=?5 AND updated_at=?6",
            params![
                id,
                next_due_date,
                status,
                updated_at,
                expected_due_date,
                expected_updated_at
            ],
        )?;
        if changed == 0 {
            let exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM renewals WHERE id=?1)",
                params![id],
                |row| row.get(0),
            )?;
            return Err(if exists {
                RenewalsStoreError::Conflict
            } else {
                RenewalsStoreError::NotFound
            });
        }
        tx.commit()?;
        Ok(())
    }

    pub fn delete(&self, id: i64, expected_updated_at: &str) -> Result<(), RenewalsStoreError> {
        let changed = self.connect()?.execute(
            "DELETE FROM renewals WHERE id=?1 AND updated_at=?2",
            params![id, expected_updated_at],
        )?;
        if changed == 0 {
            return Err(if self.get(id)?.is_some() {
                RenewalsStoreError::Conflict
            } else {
                RenewalsStoreError::NotFound
            });
        }
        Ok(())
    }

    pub fn backup(&self) -> Result<PathBuf, RenewalsStoreError> {
        let backup_dir = luma_next_support_dir()?.join("backups");
        std::fs::create_dir_all(&backup_dir)?;
        let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%S%3fZ");
        let destination = backup_dir.join(format!("renewals-backup-{stamp}.sqlite"));
        let temporary = backup_dir.join(format!(".renewals-backup-{stamp}.tmp"));
        if temporary.exists() {
            std::fs::remove_file(&temporary)?;
        }
        let quoted = sqlite_path_literal(&temporary)?;
        self.connect()?
            .execute_batch(&format!("VACUUM INTO {quoted}"))?;
        std::fs::rename(&temporary, &destination)?;
        Ok(destination)
    }
}

fn row_from_sql(row: &rusqlite::Row<'_>) -> rusqlite::Result<RenewalRow> {
    Ok(RenewalRow {
        id: row.get(0)?,
        name: row.get(1)?,
        category: row.get(2)?,
        amount_minor: row.get(3)?,
        currency: row.get(4)?,
        cadence_kind: row.get(5)?,
        cadence_value: row.get(6)?,
        anchor_month: row.get(7)?,
        anchor_day: row.get(8)?,
        next_due_date: row.get(9)?,
        auto_renew: row.get::<_, i64>(10)? != 0,
        status: row.get(11)?,
        url: row.get(12)?,
        note: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}

fn sqlite_path_literal(path: &Path) -> Result<String, RenewalsStoreError> {
    let text = path
        .to_str()
        .ok_or_else(|| RenewalsStoreError::Msg("backup path is not valid UTF-8".into()))?;
    if text.contains('\0') {
        return Err(RenewalsStoreError::Msg(
            "backup path contains a NUL byte".into(),
        ));
    }
    Ok(format!("'{}'", text.replace('\'', "''")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(updated: &str) -> RenewalRow {
        RenewalRow {
            id: 0,
            name: "Cloud".into(),
            category: "software".into(),
            amount_minor: Some(999),
            currency: Some("USD".into()),
            cadence_kind: "monthly".into(),
            cadence_value: None,
            anchor_month: Some(1),
            anchor_day: Some(31),
            next_due_date: "2024-01-31".into(),
            auto_renew: true,
            status: "active".into(),
            url: None,
            note: None,
            created_at: updated.into(),
            updated_at: updated.into(),
        }
    }

    #[test]
    fn round_trip_reopen_update_delete_and_paid_cas() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("renewals.sqlite");
        let store = RenewalsStore::with_path(path.clone()).unwrap();
        let row = store.insert(&sample("v1")).unwrap();
        drop(store);
        let store = RenewalsStore::with_path(path).unwrap();
        assert_eq!(store.list().unwrap(), vec![row.clone()]);
        store
            .mark_paid(row.id, "2024-01-31", "v1", "2024-02-29", "active", "v2")
            .unwrap();
        assert!(matches!(
            store.mark_paid(row.id, "2024-01-31", "v1", "x", "active", "v3"),
            Err(RenewalsStoreError::Conflict)
        ));
        store.delete(row.id, "v2").unwrap();
        assert!(store.list().unwrap().is_empty());
    }

    #[test]
    fn capacity_rejects_new_rows() {
        let temp = tempfile::tempdir().unwrap();
        let store = RenewalsStore::with_path(temp.path().join("renewals.sqlite")).unwrap();
        let conn = store.connect().unwrap();
        for index in 0..MAX_RENEWALS_ROWS {
            conn.execute(
                "INSERT INTO renewals
                 (name,category,cadence_kind,next_due_date,auto_renew,status,created_at,updated_at)
                 VALUES (?1,'x','once','2026-01-01',0,'active','x','x')",
                params![format!("r{index}")],
            )
            .unwrap();
        }
        assert!(matches!(
            store.insert(&sample("v1")),
            Err(RenewalsStoreError::Capacity)
        ));
    }

    #[test]
    fn backup_is_reopenable() {
        let temp = tempfile::tempdir().unwrap();
        let _env = crate::paths::LumaNextTestEnvGuard::override_paths(
            temp.path(),
            &temp.path().join("logs"),
        );
        let store = RenewalsStore::luma_next_default().unwrap();
        store.insert(&sample("v1")).unwrap();
        let backup = store.backup().unwrap();
        assert_eq!(
            RenewalsStore::with_path(backup)
                .unwrap()
                .list()
                .unwrap()
                .len(),
            1
        );
    }
}
