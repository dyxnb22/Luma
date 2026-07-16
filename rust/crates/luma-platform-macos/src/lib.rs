//! macOS host adapters (filesystem catalogs, pasteboard, proxy, SSH config, AX, windows, …).
//!
//! No business rules — modules call these via ports. Most code is safe I/O and parsing;
//! unsafe FFI is confined to accessibility/window helpers when the platform API requires it.

mod accessibility;
mod apps;
mod bounded_file_reader;
mod clock;
mod fs_watch;
mod keychain;
mod markdown_watch;
mod notes_workspace;
mod open_path;
mod pasteboard;
mod profile_store;
mod project_workspace;
mod proxy_core;
mod recipe_environment;
mod speech;
mod ssh_config;
mod system_proxy;
#[cfg(target_os = "macos")]
mod window;
#[cfg(not(target_os = "macos"))]
#[path = "window_stub.rs"]
mod window;

pub use accessibility::{Accessibility, AccessibilityError, FakeAccessibility, MacAccessibility};
pub use apps::{AppEntry, AppLaunchError, AppsCatalog, FilesystemAppsCatalog};
pub use bounded_file_reader::MacBoundedUtf8FileReader;
pub use clock::MacClock;
pub use fs_watch::{poll_markdown_root, watch_markdown_root, DirFingerprint};
pub use keychain::{FakeKeychain, Keychain, KeychainError, MacKeychain, SecretLabel};
pub use markdown_watch::MacMarkdownWatcher;
pub use notes_workspace::MacNotesWorkspace;
pub use open_path::{FakeOpenPath, MacOpenPath, OpenPath, OpenPathError};
pub use pasteboard::{MacPasteboard, Pasteboard, PasteboardError};
pub use profile_store::MacProfileStore;
pub use project_workspace::MacProjectWorkspace;
pub use proxy_core::MacMihomoProxyCore;
pub use recipe_environment::{MacCommandRunner, MacRecipeEnvironment};
pub use speech::MacSpeech;
pub use ssh_config::MacSshConfig;
pub use system_proxy::MacSystemProxy;
pub use window::MacWindowCatalog;
