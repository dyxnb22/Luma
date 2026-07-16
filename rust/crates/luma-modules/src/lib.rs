//! Built-in modules (personal daily driver).

mod apps;
mod cancel;
mod clipboard;
mod clipboard_privacy;
mod command_recipes;
mod fake;
mod notes;
mod ports;
mod projects;
mod proxy;
mod quicklinks;
mod records;
mod secrets;
mod snippets;
mod ssh;
mod ux;
mod windows;
mod wordbook;

pub use apps::AppsModule;
pub use clipboard::ClipboardModule;
pub use clipboard_privacy::ClipboardSuppression;
pub use command_recipes::CommandRecipesModule;
pub use fake::FakeEchoModule;
pub use notes::{NotesModule, NotesServices};
pub use ports::PortsModule;
pub use projects::ProjectsModule;
pub use proxy::ProxyModule;
pub use quicklinks::QuicklinksModule;
pub use records::RecordsModule;
pub use secrets::SecretsModule;
pub use snippets::SnippetsModule;
pub use ssh::SshModule;
pub use windows::{WindowsModule, HUB_WINDOWS_MAX};
pub use wordbook::WordbookModule;
