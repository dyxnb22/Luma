//! Interactive TUI layer. One AppState, one event loop. No DB / macOS / module I/O.

mod app;
mod effect;
mod mock_engine;
mod msg;
mod reducer;
mod render;
mod terminal;
mod theme;
mod view_model;

pub use app::run_tui_with_engine;
pub use effect::Effect;
pub use mock_engine::MockEngine;
pub use msg::Msg;
pub use reducer::update;
pub use render::render;
pub use terminal::{install_panic_hook, TerminalGuard};
pub use theme::{Symbols, Theme, ThemeMode};
pub use view_model::{AppState, Route, StatusTone};
