//! macOS adapters. No business rules; modules call these via ports.

mod accessibility;
mod apps;
mod automation;
mod eventkit;
mod fs_watch;
mod keychain;
mod open_path;
mod pasteboard;
mod process;
mod translation;

pub use accessibility::{Accessibility, AccessibilityError, FakeAccessibility, MacAccessibility};
pub use apps::{AppEntry, AppLaunchError, AppsCatalog, FilesystemAppsCatalog};
pub use automation::{
    Automation, AutomationAuth, AutomationError, BrowserTab, FakeAutomation, MacAutomation,
};
pub use eventkit::{
    EventKit, EventKitError, FakeEventKit, MacEventKit, ReminderItem, RemindersAuth,
};
pub use fs_watch::{poll_markdown_root, watch_markdown_root, DirFingerprint};
pub use keychain::{FakeKeychain, Keychain, KeychainError, MacKeychain, SecretLabel};
pub use open_path::{FakeOpenPath, MacOpenPath, OpenPath, OpenPathError};
pub use pasteboard::{MacPasteboard, Pasteboard, PasteboardError};
pub use process::{MacProcessCatalog, ProcessCatalog, ProcessEntry, ProcessError};
pub use translation::{
    FakeTranslator, MacTranslator, TranslationError, TranslationResult, Translator,
};
