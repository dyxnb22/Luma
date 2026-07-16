//! Pure domain primitives. No I/O, Tokio, filesystem, or terminal.

mod error;
mod id;
mod privacy;
mod query;
mod recipe;
mod result_item;

pub use error::{DomainError, FailureKind};
pub use id::{ActionId, ModuleId, OperationId, RequestId, ResultId};
pub use privacy::looks_secret;
pub use query::{Query, QueryScope};
pub use recipe::{
    CommandStep, ConfigIssue, Recipe, RecipeCatalog, RecipeMetadata, RecipeRisk, RecipeRunOutcome,
    RecipeRunPlan, RecipeScope, RecipeVariant, ResolvedCommandStep, StepRunResult, VariantMatch,
};
pub use result_item::{ActionDescriptor, ActionRisk, SearchItem};
