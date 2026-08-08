//! Built-in modules (personal daily driver).

mod apps;
mod cancel;
mod clipboard;
mod clipboard_privacy;
mod command_recipes;
mod fake;
mod git;
mod projects;
mod records;
mod runtime;
mod ux;
mod windows;
mod wordbook;

pub use apps::AppsModule;
pub use clipboard::ClipboardModule;
pub use clipboard_privacy::ClipboardSuppression;
pub use command_recipes::CommandRecipesModule;
pub use fake::FakeEchoModule;
pub use git::GitModule;
pub use projects::ProjectsModule;
pub use records::RecordsModule;
pub use runtime::RuntimeModule;
pub use windows::{WindowsModule, HUB_WINDOWS_MAX};
pub use wordbook::WordbookModule;
