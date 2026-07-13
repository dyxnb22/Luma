//! Built-in modules.

mod apps;
mod ax_gated;
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

use luma_application::ModuleRegistry;
use luma_platform_macos::FilesystemAppsCatalog;
use luma_storage::{ClipboardStore, ClipboardStoreError, ConfigError, ConfigStore, LumaSettings};
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Clipboard(#[from] ClipboardStoreError),
}

/// Build registry from settings + an already-opened clipboard store (no silent fallback).
pub fn registry_from_settings(
    settings: &LumaSettings,
    clipboard: Arc<ClipboardStore>,
) -> ModuleRegistry {
    let notes_root = settings.notes_root.as_ref().map(PathBuf::from);
    let project_roots: Vec<PathBuf> = settings.projects_roots.iter().map(PathBuf::from).collect();

    let mut reg = ModuleRegistry::new();
    reg.register(Arc::new(AppsModule::new(Arc::new(
        FilesystemAppsCatalog::system_default(),
    ))));
    reg.register(Arc::new(ClipboardModule::with_deps(
        clipboard,
        Arc::new(luma_platform_macos::MacPasteboard),
        Arc::new(luma_platform_macos::MacAccessibility),
    )));
    reg.register(Arc::new(NotesModule::with_root(notes_root)));
    reg.register(Arc::new(QuicklinksModule::new()));
    reg.register(Arc::new(SnippetsModule::new()));
    reg.register(Arc::new(TranslateModule::new()));
    reg.register(Arc::new(TodoModule::new()));
    reg.register(Arc::new(ProjectsModule::with_roots(project_roots)));
    reg.register(Arc::new(KillProcessModule::new()));
    reg.register(Arc::new(MediaModule::new()));
    reg.register(Arc::new(WordbookModule::new()));
    reg.register(Arc::new(WindowLayoutsModule::new()));
    reg.register(Arc::new(MenuItemsModule::new()));
    reg.register(Arc::new(BrowserTabsModule::new()));
    reg.register(Arc::new(SecretsModule::new()));
    reg.register(Arc::new(FakeEchoModule::new()));

    for (id, enabled) in &settings.enabled_modules {
        let _ = reg.set_enabled(id, *enabled);
    }
    // Fake stays off unless explicitly enabled in settings (tests / soak).
    if !settings
        .enabled_modules
        .get("luma.fake")
        .copied()
        .unwrap_or(false)
    {
        let _ = reg.set_enabled("luma.fake", false);
    }
    reg
}

/// Load LumaNext settings + clipboard. Corrupt config is not replaced with defaults.
pub fn load_registry() -> Result<ModuleRegistry, RegistryError> {
    let store = ConfigStore::luma_next_default()?;
    let settings = store.load_or_default()?;
    let clipboard = Arc::new(ClipboardStore::luma_next_default()?);
    Ok(registry_from_settings(&settings, clipboard))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn corrupt_config_does_not_silently_default() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        fs::write(&path, "this is not toml {{{").unwrap();
        let store = ConfigStore::with_path(path.clone());
        let err = store.load_or_default().unwrap_err();
        match err {
            ConfigError::Corrupt(q) => {
                assert!(q.exists());
                assert!(!path.exists(), "corrupt settings must not remain in place");
            }
            other => panic!("expected Corrupt, got {other:?}"),
        }
        // Must not invent a default registry that re-enables modules.
        assert!(load_registry_from_store(&store).is_err());
    }

    fn load_registry_from_store(store: &ConfigStore) -> Result<ModuleRegistry, RegistryError> {
        let settings = store.load_or_default()?;
        let dir = tempdir().unwrap();
        let clipboard = Arc::new(ClipboardStore::with_path(dir.path().join("c.sqlite")).unwrap());
        Ok(registry_from_settings(&settings, clipboard))
    }
}
