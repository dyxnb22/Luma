use crate::ports::{CommandRecipesRepoError, CommandRecipesRepository};
use luma_domain::{RecipeCatalog, RecipeMetadata, RecipeRunOutcome};
use luma_storage::{command_recipes_config_path, load_recipe_catalog, CommandRecipesMetaStore};
use std::path::PathBuf;
use std::sync::Arc;

pub struct SqliteCommandRecipesRepository {
    meta: Arc<CommandRecipesMetaStore>,
    support_dir: PathBuf,
}

impl SqliteCommandRecipesRepository {
    pub fn new(meta: Arc<CommandRecipesMetaStore>, support_dir: PathBuf) -> Self {
        Self { meta, support_dir }
    }
}

impl CommandRecipesRepository for SqliteCommandRecipesRepository {
    fn load_catalog(&self) -> RecipeCatalog {
        load_recipe_catalog(&command_recipes_config_path(&self.support_dir))
    }

    fn get_metadata(&self, recipe_id: &str) -> Result<RecipeMetadata, CommandRecipesRepoError> {
        self.meta
            .get(recipe_id)
            .map_err(|e| CommandRecipesRepoError::msg(e.to_string()))
    }

    fn set_favorite(&self, recipe_id: &str, favorite: bool) -> Result<(), CommandRecipesRepoError> {
        self.meta
            .set_favorite(recipe_id, favorite)
            .map_err(|e| CommandRecipesRepoError::msg(e.to_string()))
    }

    fn record_run(
        &self,
        recipe_id: &str,
        result: RecipeRunOutcome,
        now_unix: i64,
    ) -> Result<(), CommandRecipesRepoError> {
        self.meta
            .record_run(recipe_id, result, now_unix)
            .map_err(|e| CommandRecipesRepoError::msg(e.to_string()))
    }

    fn config_path(&self) -> Option<PathBuf> {
        Some(command_recipes_config_path(&self.support_dir))
    }
}

#[derive(Default)]
pub struct MemoryCommandRecipesRepository {
    catalog: RecipeCatalog,
    metadata: std::sync::Mutex<std::collections::BTreeMap<String, RecipeMetadata>>,
    config_path: Option<PathBuf>,
}

impl MemoryCommandRecipesRepository {
    pub fn with_catalog(catalog: RecipeCatalog) -> Self {
        Self {
            catalog,
            metadata: std::sync::Mutex::new(std::collections::BTreeMap::new()),
            config_path: None,
        }
    }

    pub fn set_config_path(mut self, path: PathBuf) -> Self {
        self.config_path = Some(path);
        self
    }
}

impl CommandRecipesRepository for MemoryCommandRecipesRepository {
    fn load_catalog(&self) -> RecipeCatalog {
        self.catalog.clone()
    }

    fn get_metadata(&self, recipe_id: &str) -> Result<RecipeMetadata, CommandRecipesRepoError> {
        Ok(self
            .metadata
            .lock()
            .expect("recipe meta lock")
            .get(recipe_id)
            .cloned()
            .unwrap_or_default())
    }

    fn set_favorite(&self, recipe_id: &str, favorite: bool) -> Result<(), CommandRecipesRepoError> {
        self.metadata
            .lock()
            .expect("recipe meta lock")
            .entry(recipe_id.to_string())
            .or_default()
            .favorite = favorite;
        Ok(())
    }

    fn record_run(
        &self,
        recipe_id: &str,
        result: RecipeRunOutcome,
        now_unix: i64,
    ) -> Result<(), CommandRecipesRepoError> {
        let mut guard = self.metadata.lock().expect("recipe meta lock");
        let entry = guard.entry(recipe_id.to_string()).or_default();
        entry.last_used_at = Some(now_unix);
        entry.use_count = entry.use_count.saturating_add(1);
        entry.last_result = Some(result);
        Ok(())
    }

    fn config_path(&self) -> Option<PathBuf> {
        self.config_path.clone()
    }
}
