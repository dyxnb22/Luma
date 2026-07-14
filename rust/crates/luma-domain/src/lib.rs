//! Pure domain primitives. No I/O, Tokio, filesystem, or terminal.

mod error;
mod id;
mod privacy;
mod query;
mod result_item;

pub use error::{DomainError, FailureKind};
pub use id::{ActionId, ModuleId, OperationId, RequestId, ResultId};
pub use privacy::looks_secret;
pub use query::{Query, QueryScope};
pub use result_item::{ActionDescriptor, ActionRisk, SearchItem};
