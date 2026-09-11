use luma_domain::{RecipeCatalog, RecipeMetadata, RecipeRunOutcome};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{0}")]
pub struct CommandRecipesRepoError(pub String);

impl CommandRecipesRepoError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

pub trait CommandRecipesRepository: Send + Sync {
    fn load_catalog(&self) -> RecipeCatalog;
    fn get_metadata(&self, recipe_id: &str) -> Result<RecipeMetadata, CommandRecipesRepoError>;
    fn set_favorite(&self, recipe_id: &str, favorite: bool) -> Result<(), CommandRecipesRepoError>;
    fn record_run(
        &self,
        recipe_id: &str,
        result: RecipeRunOutcome,
        now_unix: i64,
    ) -> Result<(), CommandRecipesRepoError>;
    fn config_path(&self) -> Option<std::path::PathBuf>;
}
