//! Built-in modules.

mod apps;
mod ax_gated;
mod cancel;
mod clipboard;
mod fake;
mod kill;
mod media;
mod notes;
mod projects;
mod quicklinks;
mod secrets;
mod snippets;
mod todo;
mod translate;
mod wordbook;

pub use apps::AppsModule;
pub use ax_gated::{BrowserTabsModule, MenuItemsModule, WindowLayoutsModule};
pub use clipboard::ClipboardModule;
pub use fake::FakeEchoModule;
pub use kill::KillProcessModule;
pub use media::MediaModule;
pub use notes::NotesModule;
pub use projects::ProjectsModule;
pub use quicklinks::QuicklinksModule;
pub use secrets::SecretsModule;
pub use snippets::SnippetsModule;
pub use todo::TodoModule;
pub use translate::TranslateModule;
pub use wordbook::WordbookModule;
