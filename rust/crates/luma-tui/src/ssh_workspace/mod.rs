//! SSH Workspace TUI surface (embedded terminal + command shelf).

pub mod screen;
pub mod state;

pub use screen::{VtScreen, SCROLLBACK_CAP};
pub use state::{SshConnectionPhase, SshWorkspaceFocus, SshWorkspaceState};
