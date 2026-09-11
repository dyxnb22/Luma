use super::super::*;
use tracing::warn;

impl Engine {
    pub(crate) async fn handle_record_recipe_run(
        &self,
        recipe_id: String,
        result: luma_domain::RecipeRunOutcome,
        now_unix: i64,
    ) {
        if let Some(repo) = &self.command_recipes {
            if let Err(err) = repo.record_run(&recipe_id, result, now_unix) {
                warn!(
                    recipe_id = %recipe_id,
                    error = %err,
                    "failed to record recipe run"
                );
            }
        }
    }
}
