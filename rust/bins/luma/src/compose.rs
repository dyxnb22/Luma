//! Sole composition root helpers: wire settings, stores, and platform adapters into a registry.

use luma_application::ModuleRegistry;
use luma_modules::{
    AppsModule, BrowserTabsModule, ClipboardModule, FakeEchoModule, KillProcessModule, MediaModule,
    MenuItemsModule, NotesModule, ProjectsModule, QuicklinksModule, SecretsModule, SnippetsModule,
    TodoModule, TranslateModule, WindowLayoutsModule, WordbookModule,
};
use luma_platform_macos::{
    FilesystemAppsCatalog, MacAccessibility, MacEventKit, MacKeychain, MacOpenPath, MacPasteboard,
    MacProcessCatalog,
};
use luma_storage::{
    ClipboardStore, ClipboardStoreError, ConfigError, ConfigStore, LumaSettings, QuicklinksStore,
    QuicklinksStoreError, SnippetsStore, SnippetsStoreError,
};
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Clipboard(#[from] ClipboardStoreError),
    #[error(transparent)]
    Quicklinks(#[from] QuicklinksStoreError),
    #[error(transparent)]
    Snippets(#[from] SnippetsStoreError),
}

/// Build registry from settings + already-opened stores (no silent fallback).
/// All production adapters/stores are injected here — modules do not self-wire.
pub fn registry_from_settings(
    settings: &LumaSettings,
    clipboard: Arc<ClipboardStore>,
    quicklinks: Arc<QuicklinksStore>,
    snippets: Arc<SnippetsStore>,
) -> ModuleRegistry {
    let notes_root = settings.notes_root.as_ref().map(PathBuf::from);
    let project_roots: Vec<PathBuf> = settings.projects_roots.iter().map(PathBuf::from).collect();

    let opener = Arc::new(MacOpenPath);
    let pasteboard = Arc::new(MacPasteboard);
    let accessibility = Arc::new(MacAccessibility);

    let mut reg = ModuleRegistry::new();
    reg.register(Arc::new(AppsModule::new(Arc::new(
        FilesystemAppsCatalog::system_default(),
    ))));
    reg.register(Arc::new(ClipboardModule::with_deps(
        clipboard,
        pasteboard.clone(),
        accessibility.clone(),
    )));
    reg.register(Arc::new(NotesModule::with_root(notes_root, opener.clone())));
    reg.register(Arc::new(QuicklinksModule::with_deps(
        quicklinks,
        opener.clone(),
    )));
    reg.register(Arc::new(SnippetsModule::with_store(
        snippets,
        pasteboard.clone(),
        accessibility,
    )));
    reg.register(Arc::new(TranslateModule::new()));
    reg.register(Arc::new(TodoModule::with_eventkit(Arc::new(MacEventKit))));
    reg.register(Arc::new(ProjectsModule::with_roots(project_roots)));
    reg.register(Arc::new(KillProcessModule::with_catalog(Arc::new(
        MacProcessCatalog,
    ))));
    reg.register(Arc::new(MediaModule::new()));
    reg.register(Arc::new(WordbookModule::new()));
    reg.register(Arc::new(WindowLayoutsModule::new()));
    reg.register(Arc::new(MenuItemsModule::new()));
    reg.register(Arc::new(BrowserTabsModule::new()));
    reg.register(Arc::new(SecretsModule::with_deps(
        Arc::new(MacKeychain::luma_next()),
        pasteboard,
    )));
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

/// Load LumaNext settings + stores. Corrupt config is not replaced with defaults.
pub fn load_registry() -> Result<ModuleRegistry, RegistryError> {
    let store = ConfigStore::luma_next_default()?;
    let settings = store.load_or_default()?;
    let clipboard = Arc::new(ClipboardStore::luma_next_default()?);
    let quicklinks = Arc::new(QuicklinksStore::luma_next_default()?);
    let snippets = Arc::new(SnippetsStore::luma_next_default()?);
    Ok(registry_from_settings(
        &settings, clipboard, quicklinks, snippets,
    ))
}
