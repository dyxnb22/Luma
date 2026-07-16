use crate::paths::{ensure_luma_next_dirs, luma_next_support_dir, PathsError};
use rusqlite::{params, Connection};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PortsMetaStoreError {
    #[error(transparent)]
    Paths(#[from] PathsError),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortMetaRow {
    pub port: u16,
    pub display_name: Option<String>,
    pub favorite: bool,
    pub last_seen_at: Option<String>,
    pub kill_count: i64,
}

pub struct PortsMetaStore {
    path: PathBuf,
}

impl PortsMetaStore {
    pub fn luma_next_default() -> Result<Self, PortsMetaStoreError> {
        ensure_luma_next_dirs()?;
        Self::with_path(luma_next_support_dir()?.join("ports_meta.sqlite"))
    }

    pub fn with_path(path: PathBuf) -> Result<Self, PortsMetaStoreError> {
        let store = Self { path };
        store.init()?;
        Ok(store)
    }

    fn connect(&self) -> Result<Connection, PortsMetaStoreError> {
        crate::sqlite::open_connection(&self.path).map_err(Into::into)
    }

    fn init(&self) -> Result<(), PortsMetaStoreError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        self.connect()?.execute_batch(
            "CREATE TABLE IF NOT EXISTS port_meta (
                port INTEGER PRIMARY KEY,
                display_name TEXT,
                favorite INTEGER NOT NULL DEFAULT 0,
                last_seen_at TEXT,
                kill_count INTEGER NOT NULL DEFAULT 0
            );",
        )?;
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<PortMetaRow>, PortsMetaStoreError> {
        let conn = self.connect()?;
        let mut statement = conn.prepare(
            "SELECT port, display_name, favorite, last_seen_at, kill_count
             FROM port_meta ORDER BY port",
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok(PortMetaRow {
                    port: row.get::<_, i64>(0)? as u16,
                    display_name: row.get(1)?,
                    favorite: row.get::<_, i64>(2)? != 0,
                    last_seen_at: row.get(3)?,
                    kill_count: row.get(4)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    pub fn get(&self, port: u16) -> Result<Option<PortMetaRow>, PortsMetaStoreError> {
        let conn = self.connect()?;
        let mut statement = conn.prepare(
            "SELECT port, display_name, favorite, last_seen_at, kill_count
             FROM port_meta WHERE port = ?1",
        )?;
        let mut rows = statement.query(params![i64::from(port)])?;
        if let Some(row) = rows.next()? {
            return Ok(Some(PortMetaRow {
                port: row.get::<_, i64>(0)? as u16,
                display_name: row.get(1)?,
                favorite: row.get::<_, i64>(2)? != 0,
                last_seen_at: row.get(3)?,
                kill_count: row.get(4)?,
            }));
        }
        Ok(None)
    }

    pub fn ensure_row(&self, port: u16) -> Result<(), PortsMetaStoreError> {
        self.connect()?.execute(
            "INSERT INTO port_meta (port) VALUES (?1)
             ON CONFLICT(port) DO NOTHING",
            params![i64::from(port)],
        )?;
        Ok(())
    }

    pub fn set_display_name(
        &self,
        port: u16,
        display_name: Option<&str>,
    ) -> Result<(), PortsMetaStoreError> {
        self.ensure_row(port)?;
        let cleaned = display_name
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        self.connect()?.execute(
            "UPDATE port_meta SET display_name = ?2 WHERE port = ?1",
            params![i64::from(port), cleaned],
        )?;
        Ok(())
    }

    pub fn set_favorite(&self, port: u16, favorite: bool) -> Result<(), PortsMetaStoreError> {
        self.ensure_row(port)?;
        self.connect()?.execute(
            "UPDATE port_meta SET favorite = ?2 WHERE port = ?1",
            params![i64::from(port), if favorite { 1 } else { 0 }],
        )?;
        Ok(())
    }

    pub fn record_seen(&self, port: u16, seen_at: &str) -> Result<(), PortsMetaStoreError> {
        self.ensure_row(port)?;
        self.connect()?.execute(
            "UPDATE port_meta SET last_seen_at = ?2 WHERE port = ?1",
            params![i64::from(port), seen_at],
        )?;
        Ok(())
    }

    pub fn record_kill(&self, port: u16, killed_at: &str) -> Result<(), PortsMetaStoreError> {
        self.ensure_row(port)?;
        self.connect()?.execute(
            "UPDATE port_meta
             SET kill_count = kill_count + 1, last_seen_at = ?2
             WHERE port = ?1",
            params![i64::from(port), killed_at],
        )?;
        Ok(())
    }

    pub fn delete(&self, port: u16) -> Result<(), PortsMetaStoreError> {
        self.connect()?.execute(
            "DELETE FROM port_meta WHERE port = ?1",
            params![i64::from(port)],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn upsert_name_favorite_and_kill_round_trip() {
        let dir = tempdir().unwrap();
        let store = PortsMetaStore::with_path(dir.path().join("ports_meta.sqlite")).unwrap();
        store.set_display_name(3000, Some("api")).unwrap();
        store.set_favorite(3000, true).unwrap();
        store.record_seen(3000, "2026-07-16T10:00:00Z").unwrap();
        store.record_kill(3000, "2026-07-16T10:01:00Z").unwrap();
        let row = store.get(3000).unwrap().unwrap();
        assert_eq!(row.display_name.as_deref(), Some("api"));
        assert!(row.favorite);
        assert_eq!(row.kill_count, 1);
        assert_eq!(row.last_seen_at.as_deref(), Some("2026-07-16T10:01:00Z"));
        store.delete(3000).unwrap();
        assert!(store.get(3000).unwrap().is_none());
    }

    #[test]
    fn blank_display_name_clears() {
        let dir = tempdir().unwrap();
        let store = PortsMetaStore::with_path(dir.path().join("ports_meta.sqlite")).unwrap();
        store.set_display_name(8080, Some("web")).unwrap();
        store.set_display_name(8080, Some("  ")).unwrap();
        let row = store.get(8080).unwrap().unwrap();
        assert!(row.display_name.is_none());
    }
}
