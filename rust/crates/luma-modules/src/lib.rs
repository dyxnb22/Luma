//! Built-in modules (personal daily driver).

mod apps;
mod cancel;
mod clipboard;
mod clipboard_privacy;
mod fake;
mod notes;
mod projects;
mod proxy;
mod quicklinks;
mod records;
mod secrets;
mod snippets;
mod ux;
mod windows;
mod wordbook;

pub use apps::AppsModule;
pub use clipboard::ClipboardModule;
pub use clipboard_privacy::ClipboardSuppression;
pub use fake::FakeEchoModule;
pub use notes::{NotesModule, NotesServices};
pub use projects::ProjectsModule;
pub use proxy::ProxyModule;
pub use quicklinks::QuicklinksModule;
pub use records::RecordsModule;
pub use secrets::SecretsModule;
pub use snippets::SnippetsModule;
pub use windows::{WindowsModule, HUB_WINDOWS_MAX};
pub use wordbook::WordbookModule;
