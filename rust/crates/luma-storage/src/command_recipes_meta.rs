use crate::paths::{ensure_luma_next_dirs, luma_next_support_dir, PathsError};
use luma_domain::{RecipeMetadata, RecipeRunOutcome};
use rusqlite::{params, Connection};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CommandRecipesMetaError {
    #[error(transparent)]
    Paths(#[from] PathsError),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

pub struct CommandRecipesMetaStore {
    path: PathBuf,
}

impl CommandRecipesMetaStore {
    pub fn luma_next_default() -> Result<Self, CommandRecipesMetaError> {
        ensure_luma_next_dirs()?;
        Self::with_path(luma_next_support_dir()?.join("command-recipes-meta.sqlite"))
    }

    pub fn with_path(path: PathBuf) -> Result<Self, CommandRecipesMetaError> {
        let store = Self { path };
        store.init()?;
        Ok(store)
    }

    fn connect(&self) -> Result<Connection, CommandRecipesMetaError> {
        crate::sqlite::open_connection(&self.path).map_err(Into::into)
    }

    fn init(&self) -> Result<(), CommandRecipesMetaError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        self.connect()?.execute_batch(
            "CREATE TABLE IF NOT EXISTS recipe_meta (
                recipe_id TEXT PRIMARY KEY,
                favorite INTEGER NOT NULL DEFAULT 0,
                last_used_at INTEGER,
                use_count INTEGER NOT NULL DEFAULT 0,
                last_result TEXT
            );",
        )?;
        Ok(())
    }

    pub fn get(&self, recipe_id: &str) -> Result<RecipeMetadata, CommandRecipesMetaError> {
        let conn = self.connect()?;
        let mut stmt =
            conn.prepare("SELECT favorite, last_used_at, use_count, last_result FROM recipe_meta WHERE recipe_id = ?1")?;
        let mut rows = stmt.query(params![recipe_id])?;
        if let Some(row) = rows.next()? {
            let favorite: i64 = row.get(0)?;
            let last_used_at: Option<i64> = row.get(1)?;
            let use_count: i64 = row.get(2)?;
            let last_result: Option<String> = row.get(3)?;
            return Ok(RecipeMetadata {
                favorite: favorite != 0,
                last_used_at,
                use_count: u64::try_from(use_count.max(0)).unwrap_or(0),
                last_result: last_result.and_then(|s| match s.as_str() {
                    "success" => Some(RecipeRunOutcome::Success),
                    "failed" => Some(RecipeRunOutcome::Failed),
                    "cancelled" => Some(RecipeRunOutcome::Cancelled),
                    _ => None,
                }),
            });
        }
        Ok(RecipeMetadata::default())
    }

    pub fn set_favorite(
        &self,
        recipe_id: &str,
        favorite: bool,
    ) -> Result<(), CommandRecipesMetaError> {
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO recipe_meta (recipe_id, favorite) VALUES (?1, ?2)
             ON CONFLICT(recipe_id) DO UPDATE SET favorite = excluded.favorite",
            params![recipe_id, i64::from(favorite)],
        )?;
        Ok(())
    }

    pub fn record_run(
        &self,
        recipe_id: &str,
        result: RecipeRunOutcome,
        now_unix: i64,
    ) -> Result<(), CommandRecipesMetaError> {
        let conn = self.connect()?;
        let result_s = match result {
            RecipeRunOutcome::Success => "success",
            RecipeRunOutcome::Failed => "failed",
            RecipeRunOutcome::Cancelled => "cancelled",
        };
        conn.execute(
            "INSERT INTO recipe_meta (recipe_id, favorite, last_used_at, use_count, last_result)
             VALUES (?1, 0, ?2, 1, ?3)
             ON CONFLICT(recipe_id) DO UPDATE SET
               last_used_at = excluded.last_used_at,
               use_count = recipe_meta.use_count + 1,
               last_result = excluded.last_result",
            params![recipe_id, now_unix, result_s],
        )?;
        Ok(())
    }

    pub fn delete_metadata(&self, recipe_id: &str) -> Result<(), CommandRecipesMetaError> {
        let conn = self.connect()?;
        conn.execute(
            "DELETE FROM recipe_meta WHERE recipe_id = ?1",
            params![recipe_id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn metadata_persists_across_reopen() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("meta.sqlite");
        {
            let store = CommandRecipesMetaStore::with_path(path.clone()).unwrap();
            store.set_favorite("test", true).unwrap();
            store
                .record_run("test", RecipeRunOutcome::Success, 1_700_000_000)
                .unwrap();
        }
        let store = CommandRecipesMetaStore::with_path(path).unwrap();
        let meta = store.get("test").unwrap();
        assert!(meta.favorite);
        assert_eq!(meta.use_count, 1);
        assert_eq!(meta.last_result, Some(RecipeRunOutcome::Success));
    }

    #[test]
    fn delete_metadata_does_not_affect_config() {
        let dir = tempdir().unwrap();
        let store = CommandRecipesMetaStore::with_path(dir.path().join("meta.sqlite")).unwrap();
        store.set_favorite("x", true).unwrap();
        store.delete_metadata("x").unwrap();
        let meta = store.get("x").unwrap();
        assert!(!meta.favorite);
        assert_eq!(meta.use_count, 0);
    }
}
