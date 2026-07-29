//! Pure domain primitives. No I/O, Tokio, filesystem, or terminal.

mod capacity;
mod error;
mod id;
mod privacy;
mod query;
mod recipe;
mod result_item;

pub use capacity::{
    MAX_CLIPBOARD_ENTRY_BYTES, MAX_PINNED_CLIPBOARD_ROWS, MAX_QUICKLINKS,
    MAX_QUICKLINK_TRIGGER_BYTES, MAX_QUICKLINK_URL_BYTES, MAX_SNIPPETS, MAX_SNIPPET_BODY_BYTES,
    MAX_SNIPPET_TRIGGER_BYTES, MAX_SSH_METADATA_ROWS, MAX_TIMERS, MAX_UNPINNED_CLIPBOARD_ROWS,
};
pub use error::{DomainError, FailureKind};
pub use id::{ActionId, ModuleId, OperationId, RequestId, ResultId};
pub use privacy::looks_secret;
pub use query::{strip_command_prefix, Query, QueryScope};
pub use recipe::{
    render_remote_command, shell_quote, CommandStep, ConfigIssue, Recipe, RecipeCatalog,
    RecipeMetadata, RecipeParameter, RecipeParameterKind, RecipeRenderError, RecipeRisk,
    RecipeRunOutcome, RecipeRunPlan, RecipeScope, RecipeTarget, RecipeVariant, ResolvedCommandStep,
    SshRecipeContext, StepRunResult, VariantMatch,
};
pub use result_item::{action_needs_confirmation, ActionDescriptor, ActionRisk, SearchItem};
