//! Boundary DTOs. Domain types are converted explicitly — never exposed raw.

mod command;
mod envelope;
mod event;

pub use command::Command;
pub use envelope::{Envelope, PROTOCOL_VERSION};
pub use event::{ActionOutcomeDto, Event, SearchItemDto};
