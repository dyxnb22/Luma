//! macOS host adapters (filesystem catalogs, pasteboard, proxy, AX, windows, …).
//!
//! No business rules — modules call these via ports. Most code is safe I/O and parsing;
//! unsafe FFI is confined to accessibility/window helpers when the platform API requires it.

mod accessibility;
mod apps;
mod bounded_file_reader;
mod git;
mod open_path;
mod pasteboard;
mod project_workspace;
mod recipe_environment;
mod runtime;
mod speech;
mod system_settings;
#[cfg(target_os = "macos")]
mod window;
#[cfg(not(target_os = "macos"))]
#[path = "window_stub.rs"]
mod window;

pub use accessibility::{Accessibility, AccessibilityError, FakeAccessibility, MacAccessibility};
pub use apps::{AppEntry, AppLaunchError, AppsCatalog, FilesystemAppsCatalog};
pub use bounded_file_reader::MacBoundedUtf8FileReader;
pub use git::MacGitRepository;
pub use open_path::{FakeOpenPath, MacOpenPath, OpenPath, OpenPathError};
pub use pasteboard::{MacPasteboard, Pasteboard, PasteboardError};
pub use project_workspace::MacProjectWorkspace;
pub use recipe_environment::{MacCommandRunner, MacRecipeEnvironment};
pub use runtime::MacRuntimeInspector;
pub use speech::MacSpeech;
pub use system_settings::MacSystemSettings;
pub use window::MacWindowCatalog;
