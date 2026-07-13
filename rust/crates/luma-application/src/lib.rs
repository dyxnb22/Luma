//! Application engine. No ratatui/crossterm.

mod engine;
mod module;
mod port;
mod registry;

pub use engine::{list_modules_json, run_action, run_doctor, run_query, Engine};
pub use module::{
    ActionOutcome, ActionRequest, LumaModule, ModuleManifest, ModuleState, SearchMode, SearchSink,
    WarmupContext,
};
pub use port::EnginePort;
pub use registry::ModuleRegistry;
