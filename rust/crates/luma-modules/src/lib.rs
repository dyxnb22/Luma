//! Built-in modules (personal daily driver).

mod apps;
mod cancel;
mod clipboard;
mod clipboard_privacy;
mod fake;
mod kill;
mod notes;
mod projects;
mod quicklinks;
mod secrets;
mod snippets;

pub use apps::AppsModule;
pub use clipboard::ClipboardModule;
pub use clipboard_privacy::ClipboardSuppression;
pub use fake::FakeEchoModule;
pub use kill::KillProcessModule;
pub use notes::NotesModule;
pub use projects::ProjectsModule;
pub use quicklinks::QuicklinksModule;
pub use secrets::SecretsModule;
pub use snippets::SnippetsModule;
