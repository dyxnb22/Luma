//! SSH Workspace TUI surface (embedded terminal + command shelf).

pub mod screen;
pub mod shelf;
pub mod state;

pub use screen::{VtScreen, SCROLLBACK_CAP};
pub use shelf::{ShelfItem, ShelfItemKind, ShelfState};
pub use state::{SshConnectionPhase, SshWorkspaceFocus, SshWorkspaceState};
