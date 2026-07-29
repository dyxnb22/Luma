//! macOS host adapters (filesystem catalogs, pasteboard, proxy, SSH config, AX, windows, …).
//!
//! No business rules — modules call these via ports. Most code is safe I/O and parsing;
//! unsafe FFI is confined to accessibility/window helpers when the platform API requires it.

mod accessibility;
mod apps;
mod bounded_file_reader;
mod clock;
mod databases;
mod downloads;
mod embedded_pty;
mod git;
mod keychain;
mod network_probe;
mod open_path;
mod packages;
mod pasteboard;
mod profile_store;
mod project_workspace;
mod proxy_core;
mod recipe_environment;
mod runtime;
mod screen_ocr;
mod shell_history;
mod shortcuts;
mod speech;
mod ssh_config;
mod system_proxy;
mod system_settings;
#[cfg(target_os = "macos")]
mod window;
#[cfg(not(target_os = "macos"))]
#[path = "window_stub.rs"]
mod window;

pub use accessibility::{Accessibility, AccessibilityError, FakeAccessibility, MacAccessibility};
pub use apps::{AppEntry, AppLaunchError, AppsCatalog, FilesystemAppsCatalog};
pub use bounded_file_reader::MacBoundedUtf8FileReader;
pub use clock::MacClock;
pub use databases::MacDatabasePlatform;
pub use downloads::MacDownloads;
pub use embedded_pty::MacEmbeddedPty;
pub use git::MacGitRepository;
pub use keychain::{FakeKeychain, Keychain, KeychainError, MacKeychain, SecretLabel};
pub use network_probe::MacNetworkProbe;
pub use open_path::{FakeOpenPath, MacOpenPath, OpenPath, OpenPathError};
pub use packages::MacHomebrew;
pub use pasteboard::{MacPasteboard, Pasteboard, PasteboardError};
pub use profile_store::MacProfileStore;
pub use project_workspace::MacProjectWorkspace;
pub use proxy_core::MacMihomoProxyCore;
pub use recipe_environment::{MacCommandRunner, MacRecipeEnvironment};
pub use runtime::MacRuntimeInspector;
pub use screen_ocr::MacScreenOcr;
pub use shell_history::MacShellHistory;
pub use shortcuts::MacShortcuts;
pub use speech::MacSpeech;
pub use ssh_config::MacSshConfig;
pub use system_proxy::MacSystemProxy;
pub use system_settings::MacSystemSettings;
pub use window::MacWindowCatalog;
