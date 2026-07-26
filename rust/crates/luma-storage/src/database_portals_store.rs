use crate::paths::{ensure_luma_next_dirs, luma_next_support_dir, PathsError};
use rusqlite::{params, Connection, ErrorCode, OptionalExtension};
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const MAX_DATABASE_PORTAL_ROWS: usize = 500;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatabasePortalRow {
    pub id: i64,
    pub label: String,
    pub kind: String,
    pub sqlite_path: Option<String>,
    pub pg_host: Option<String>,
    pub pg_port: Option<u16>,
    pub pg_database: Option<String>,
    pub pg_username: Option<String>,
    pub environment: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Error)]
pub enum DatabasePortalsStoreError {
    #[error(transparent)]
    Paths(#[from] PathsError),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("database portal not found")]
    NotFound,
    #[error("database portal changed since it was shown")]
    Conflict,
    #[error("database portal label already exists")]
    Duplicate,
    #[error("database portals capacity reached ({MAX_DATABASE_PORTAL_ROWS})")]
    Capacity,
    #[error("{0}")]
    Msg(String),
}

pub struct DatabasePortalsStore {
    path: PathBuf,
}

impl DatabasePortalsStore {
    pub fn luma_next_default() -> Result<Self, DatabasePortalsStoreError> {
        ensure_luma_next_dirs()?;
        Self::with_path(luma_next_support_dir()?.join("database_portals.sqlite"))
    }

    pub fn with_path(path: PathBuf) -> Result<Self, DatabasePortalsStoreError> {
        let store = Self { path };
        store.init()?;
        Ok(store)
    }

    fn connect(&self) -> Result<Connection, DatabasePortalsStoreError> {
        crate::sqlite::open_connection(&self.path).map_err(Into::into)
    }

    fn init(&self) -> Result<(), DatabasePortalsStoreError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        self.connect()?.execute_batch(
            "CREATE TABLE IF NOT EXISTS database_portals (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                label TEXT NOT NULL COLLATE NOCASE UNIQUE,
                kind TEXT NOT NULL CHECK(kind IN ('sqlite','postgres')),
                sqlite_path TEXT,
                pg_host TEXT,
                pg_port INTEGER,
                pg_database TEXT,
                pg_username TEXT,
                environment TEXT NOT NULL
                  CHECK(environment IN ('local','development','staging','production')),
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                CHECK(
                  (kind='sqlite' AND sqlite_path IS NOT NULL AND pg_host IS NULL
                    AND pg_port IS NULL AND pg_database IS NULL AND pg_username IS NULL)
                  OR
                  (kind='postgres' AND sqlite_path IS NULL AND pg_host IS NOT NULL
                    AND pg_port IS NOT NULL AND pg_database IS NOT NULL AND pg_username IS NOT NULL)
                )
             );
             CREATE INDEX IF NOT EXISTS idx_database_portals_label
             ON database_portals(label COLLATE NOCASE, id);",
        )?;
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<DatabasePortalRow>, DatabasePortalsStoreError> {
        let conn = self.connect()?;
        let mut statement = conn.prepare(
            "SELECT id,label,kind,sqlite_path,pg_host,pg_port,pg_database,pg_username,
                    environment,created_at,updated_at
             FROM database_portals
             ORDER BY label COLLATE NOCASE ASC, id ASC
             LIMIT ?1",
        )?;
        let rows: Result<Vec<_>, rusqlite::Error> = statement
            .query_map(params![(MAX_DATABASE_PORTAL_ROWS + 1) as i64], row_from_sql)?
            .collect();
        Ok(rows?)
    }

    pub fn get(&self, id: i64) -> Result<Option<DatabasePortalRow>, DatabasePortalsStoreError> {
        self.connect()?
            .query_row(
                "SELECT id,label,kind,sqlite_path,pg_host,pg_port,pg_database,pg_username,
                        environment,created_at,updated_at
                 FROM database_portals WHERE id=?1",
                params![id],
                row_from_sql,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn insert(
        &self,
        row: &DatabasePortalRow,
    ) -> Result<DatabasePortalRow, DatabasePortalsStoreError> {
        let conn = self.connect()?;
        let tx = conn.unchecked_transaction()?;
        let count: i64 = tx.query_row("SELECT COUNT(*) FROM database_portals", [], |row| {
            row.get(0)
        })?;
        if count >= MAX_DATABASE_PORTAL_ROWS as i64 {
            return Err(DatabasePortalsStoreError::Capacity);
        }
        let result = tx.execute(
            "INSERT INTO database_portals (
                label,kind,sqlite_path,pg_host,pg_port,pg_database,pg_username,
                environment,created_at,updated_at
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                row.label,
                row.kind,
                row.sqlite_path,
                row.pg_host,
                row.pg_port,
                row.pg_database,
                row.pg_username,
                row.environment,
                row.created_at,
                row.updated_at,
            ],
        );
        if let Err(error) = result {
            if error.sqlite_error_code() == Some(ErrorCode::ConstraintViolation) {
                let duplicate = tx.query_row(
                    "SELECT EXISTS(
                       SELECT 1 FROM database_portals WHERE label=?1 COLLATE NOCASE
                     )",
                    params![row.label],
                    |row| row.get::<_, bool>(0),
                )?;
                if duplicate {
                    return Err(DatabasePortalsStoreError::Duplicate);
                }
            }
            return Err(error.into());
        }
        let id = tx.last_insert_rowid();
        tx.commit()?;
        let mut stored = row.clone();
        stored.id = id;
        Ok(stored)
    }

    pub fn delete(
        &self,
        id: i64,
        expected_updated_at: &str,
    ) -> Result<(), DatabasePortalsStoreError> {
        let changed = self.connect()?.execute(
            "DELETE FROM database_portals WHERE id=?1 AND updated_at=?2",
            params![id, expected_updated_at],
        )?;
        self.classify_cas(id, changed)
    }

    fn classify_cas(&self, id: i64, changed: usize) -> Result<(), DatabasePortalsStoreError> {
        if changed > 0 {
            return Ok(());
        }
        Err(if self.get(id)?.is_some() {
            DatabasePortalsStoreError::Conflict
        } else {
            DatabasePortalsStoreError::NotFound
        })
    }

    pub fn backup(&self) -> Result<PathBuf, DatabasePortalsStoreError> {
        let backup_dir = luma_next_support_dir()?.join("backups");
        std::fs::create_dir_all(&backup_dir)?;
        let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%S%3fZ");
        let destination = backup_dir.join(format!("database-portals-backup-{stamp}.sqlite"));
        let temporary = backup_dir.join(format!(".database-portals-backup-{stamp}.tmp"));
        let quoted = sqlite_path_literal(&temporary)?;
        self.connect()?
            .execute_batch(&format!("VACUUM INTO {quoted}"))?;
        std::fs::rename(&temporary, &destination)?;
        Ok(destination)
    }
}

fn row_from_sql(row: &rusqlite::Row<'_>) -> rusqlite::Result<DatabasePortalRow> {
    let pg_port = row
        .get::<_, Option<i64>>(5)?
        .map(|port| {
            u16::try_from(port).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(5, port))
        })
        .transpose()?;
    Ok(DatabasePortalRow {
        id: row.get(0)?,
        label: row.get(1)?,
        kind: row.get(2)?,
        sqlite_path: row.get(3)?,
        pg_host: row.get(4)?,
        pg_port,
        pg_database: row.get(6)?,
        pg_username: row.get(7)?,
        environment: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn sqlite_path_literal(path: &Path) -> Result<String, DatabasePortalsStoreError> {
    let text = path
        .to_str()
        .ok_or_else(|| DatabasePortalsStoreError::Msg("backup path is not valid UTF-8".into()))?;
    if text.contains('\0') {
        return Err(DatabasePortalsStoreError::Msg(
            "backup path contains NUL".into(),
        ));
    }
    Ok(format!("'{}'", text.replace('\'', "''")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sqlite_row(label: &str) -> DatabasePortalRow {
        DatabasePortalRow {
            id: 0,
            label: label.into(),
            kind: "sqlite".into(),
            sqlite_path: Some("/fixture/database.sqlite".into()),
            pg_host: None,
            pg_port: None,
            pg_database: None,
            pg_username: None,
            environment: "local".into(),
            created_at: "v1".into(),
            updated_at: "v1".into(),
        }
    }

    #[test]
    fn crud_reopen_cas_and_remove_only_metadata() {
        let temp = tempfile::tempdir().unwrap();
        let database = temp.path().join("real.sqlite");
        std::fs::write(&database, b"fixture").unwrap();
        let path = temp.path().join("database_portals.sqlite");
        let mut row = sqlite_row("Local");
        row.sqlite_path = Some(database.display().to_string());
        let store = DatabasePortalsStore::with_path(path.clone()).unwrap();
        let row = store.insert(&row).unwrap();
        drop(store);
        let store = DatabasePortalsStore::with_path(path).unwrap();
        assert_eq!(store.list().unwrap(), vec![row.clone()]);
        assert!(matches!(
            store.delete(row.id, "stale"),
            Err(DatabasePortalsStoreError::Conflict)
        ));
        store.delete(row.id, "v1").unwrap();
        assert!(database.exists());
    }

    #[test]
    fn duplicate_cap_and_schema_never_have_password_columns() {
        let temp = tempfile::tempdir().unwrap();
        let store =
            DatabasePortalsStore::with_path(temp.path().join("database_portals.sqlite")).unwrap();
        store.insert(&sqlite_row("Local")).unwrap();
        assert!(matches!(
            store.insert(&sqlite_row("local")),
            Err(DatabasePortalsStoreError::Duplicate)
        ));
        let conn = store.connect().unwrap();
        let mut statement = conn.prepare("PRAGMA table_info(database_portals)").unwrap();
        let columns = statement
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(!columns.iter().any(|column| {
            column.contains("password") || column.contains("dsn") || column.contains("secret")
        }));

        let mut invalid = sqlite_row("Invalid");
        invalid.environment = "space".into();
        assert!(matches!(
            store.insert(&invalid),
            Err(DatabasePortalsStoreError::Sqlite(_))
        ));
    }

    #[test]
    fn capacity_rejects_new_metadata_rows() {
        let temp = tempfile::tempdir().unwrap();
        let store =
            DatabasePortalsStore::with_path(temp.path().join("database_portals.sqlite")).unwrap();
        let conn = store.connect().unwrap();
        for index in 0..MAX_DATABASE_PORTAL_ROWS {
            conn.execute(
                "INSERT INTO database_portals
                 (label,kind,sqlite_path,environment,created_at,updated_at)
                 VALUES (?1,'sqlite','/fixture/db.sqlite','local','v1','v1')",
                params![format!("portal-{index}")],
            )
            .unwrap();
        }
        assert!(matches!(
            store.insert(&sqlite_row("overflow")),
            Err(DatabasePortalsStoreError::Capacity)
        ));
    }

    #[test]
    fn backup_is_reopenable() {
        let temp = tempfile::tempdir().unwrap();
        let _env = crate::paths::LumaNextTestEnvGuard::override_paths(
            temp.path(),
            &temp.path().join("logs"),
        );
        let store = DatabasePortalsStore::luma_next_default().unwrap();
        store.insert(&sqlite_row("Local")).unwrap();
        let backup = store.backup().unwrap();
        assert_eq!(
            DatabasePortalsStore::with_path(backup)
                .unwrap()
                .list()
                .unwrap()
                .len(),
            1
        );
    }
}
