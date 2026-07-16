use crate::paths::{ensure_luma_next_dirs, luma_next_support_dir, PathsError};
use rusqlite::{params, Connection};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SshMetaStoreError {
    #[error(transparent)]
    Paths(#[from] PathsError),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SshHostMetaRow {
    pub alias: String,
    pub display_name: Option<String>,
    pub favorite: bool,
    pub tags: Vec<String>,
    pub last_connected_at: Option<String>,
    pub connection_count: i64,
}

pub struct SshMetaStore {
    path: PathBuf,
}

impl SshMetaStore {
    pub fn luma_next_default() -> Result<Self, SshMetaStoreError> {
        ensure_luma_next_dirs()?;
        Self::with_path(luma_next_support_dir()?.join("ssh_meta.sqlite"))
    }

    pub fn with_path(path: PathBuf) -> Result<Self, SshMetaStoreError> {
        let store = Self { path };
        store.init()?;
        Ok(store)
    }

    fn connect(&self) -> Result<Connection, SshMetaStoreError> {
        crate::sqlite::open_connection(&self.path).map_err(Into::into)
    }

    fn init(&self) -> Result<(), SshMetaStoreError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        self.connect()?.execute_batch(
            "CREATE TABLE IF NOT EXISTS ssh_host_meta (
                alias TEXT PRIMARY KEY,
                display_name TEXT,
                favorite INTEGER NOT NULL DEFAULT 0,
                tags TEXT NOT NULL DEFAULT '[]',
                last_connected_at TEXT,
                connection_count INTEGER NOT NULL DEFAULT 0
            );",
        )?;
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<SshHostMetaRow>, SshMetaStoreError> {
        let conn = self.connect()?;
        let mut statement = conn.prepare(
            "SELECT alias, display_name, favorite, tags, last_connected_at, connection_count
             FROM ssh_host_meta ORDER BY alias",
        )?;
        let rows = statement
            .query_map([], |row| {
                let tags_json: String = row.get(3)?;
                let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
                Ok(SshHostMetaRow {
                    alias: row.get(0)?,
                    display_name: row.get(1)?,
                    favorite: row.get::<_, i64>(2)? != 0,
                    tags,
                    last_connected_at: row.get(4)?,
                    connection_count: row.get(5)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    pub fn get(&self, alias: &str) -> Result<Option<SshHostMetaRow>, SshMetaStoreError> {
        let conn = self.connect()?;
        let mut statement = conn.prepare(
            "SELECT alias, display_name, favorite, tags, last_connected_at, connection_count
             FROM ssh_host_meta WHERE alias = ?1",
        )?;
        let mut rows = statement.query(params![alias])?;
        if let Some(row) = rows.next()? {
            let tags_json: String = row.get(3)?;
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            return Ok(Some(SshHostMetaRow {
                alias: row.get(0)?,
                display_name: row.get(1)?,
                favorite: row.get::<_, i64>(2)? != 0,
                tags,
                last_connected_at: row.get(4)?,
                connection_count: row.get(5)?,
            }));
        }
        Ok(None)
    }

    pub fn upsert(
        &self,
        alias: &str,
        display_name: Option<&str>,
        favorite: bool,
        tags: &[String],
    ) -> Result<(), SshMetaStoreError> {
        let tags_json = serde_json::to_string(tags)?;
        self.connect()?.execute(
            "INSERT INTO ssh_host_meta (alias, display_name, favorite, tags)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(alias) DO UPDATE SET
               display_name = COALESCE(excluded.display_name, ssh_host_meta.display_name),
               favorite = excluded.favorite,
               tags = excluded.tags",
            params![alias, display_name, favorite as i64, tags_json],
        )?;
        Ok(())
    }

    pub fn set_favorite(&self, alias: &str, favorite: bool) -> Result<(), SshMetaStoreError> {
        self.connect()?.execute(
            "INSERT INTO ssh_host_meta (alias, favorite) VALUES (?1, ?2)
             ON CONFLICT(alias) DO UPDATE SET favorite = excluded.favorite",
            params![alias, favorite as i64],
        )?;
        Ok(())
    }

    pub fn set_display_name(
        &self,
        alias: &str,
        display_name: Option<&str>,
    ) -> Result<(), SshMetaStoreError> {
        self.connect()?.execute(
            "INSERT INTO ssh_host_meta (alias, display_name) VALUES (?1, ?2)
             ON CONFLICT(alias) DO UPDATE SET display_name = excluded.display_name",
            params![alias, display_name],
        )?;
        Ok(())
    }

    pub fn record_connection(
        &self,
        alias: &str,
        connected_at: &str,
    ) -> Result<(), SshMetaStoreError> {
        self.connect()?.execute(
            "INSERT INTO ssh_host_meta (alias, last_connected_at, connection_count)
             VALUES (?1, ?2, 1)
             ON CONFLICT(alias) DO UPDATE SET
               last_connected_at = excluded.last_connected_at,
               connection_count = ssh_host_meta.connection_count + 1",
            params![alias, connected_at],
        )?;
        Ok(())
    }

    pub fn delete(&self, alias: &str) -> Result<(), SshMetaStoreError> {
        self.connect()?
            .execute("DELETE FROM ssh_host_meta WHERE alias = ?1", params![alias])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::LumaNextTestEnvGuard;
    use tempfile::tempdir;

    #[test]
    fn favorite_and_record_round_trip() {
        let dir = tempdir().unwrap();
        let _env = LumaNextTestEnvGuard::override_paths(dir.path(), &dir.path().join("logs"));
        let store = SshMetaStore::luma_next_default().unwrap();
        store.set_favorite("prod", true).unwrap();
        store.set_display_name("prod", Some("Production")).unwrap();
        store
            .record_connection("prod", "2026-01-01T00:00:00Z")
            .unwrap();
        let row = store.get("prod").unwrap().unwrap();
        assert!(row.favorite);
        assert_eq!(row.display_name.as_deref(), Some("Production"));
        assert_eq!(row.connection_count, 1);
        store.delete("prod").unwrap();
        assert!(store.get("prod").unwrap().is_none());
    }
}
