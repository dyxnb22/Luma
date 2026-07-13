//! Sole composition root helpers: wire settings, stores, and platform adapters into a registry.

use luma_application::{
    ModuleRegistry, RegistryError as ModuleRegistryError, SettingsRepository,
    SqliteClipboardHistory, SqliteQuicklinksRepository, SqliteSnippetsRepository,
    TomlSettingsRepository,
};
use luma_modules::{
    AppsModule, ClipboardModule, ClipboardSuppression, FakeEchoModule, KillProcessModule,
    NotesModule, ProjectsModule, QuicklinksModule, SecretsModule, SnippetsModule, TodoModule,
};
use luma_platform_macos::{
    FilesystemAppsCatalog, MacAccessibility, MacEventKit, MacKeychain, MacMarkdownWatcher,
    MacOpenPath, MacPasteboard, MacProcessCatalog,
};
use luma_storage::{
    ClipboardStore, ConfigError, ConfigStore, LumaSettings, QuicklinksStore, SnippetsStore,
};
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tracing::warn;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Module(#[from] ModuleRegistryError),
}

/// Build registry from settings + optionally opened stores.
/// Missing stores skip the corresponding module instead of failing the launcher.
pub fn registry_from_settings(
    settings: &LumaSettings,
    clipboard: Option<Arc<ClipboardStore>>,
    quicklinks: Option<Arc<QuicklinksStore>>,
    snippets: Option<Arc<SnippetsStore>>,
) -> Result<ModuleRegistry, ModuleRegistryError> {
    let notes_root = settings.notes_root.as_ref().map(PathBuf::from);
    let project_roots: Vec<PathBuf> = settings.projects_roots.iter().map(PathBuf::from).collect();

    let opener = Arc::new(MacOpenPath);
    let pasteboard = Arc::new(MacPasteboard);
    let accessibility = Arc::new(MacAccessibility);
    let clipboard_suppression = Arc::new(ClipboardSuppression::new());

    let mut reg = ModuleRegistry::new();
    reg.register(Arc::new(AppsModule::new(
        Arc::new(FilesystemAppsCatalog::system_default()),
        pasteboard.clone(),
    )))?;
    if let Some(clipboard) = clipboard {
        reg.register(Arc::new(ClipboardModule::with_deps(
            Arc::new(SqliteClipboardHistory::new(clipboard)),
            pasteboard.clone(),
            accessibility.clone(),
            clipboard_suppression.clone(),
        )))?;
    } else {
        warn!("clipboard store unavailable — Clipboard module not registered");
    }
    reg.register(Arc::new(NotesModule::with_root(
        notes_root,
        opener.clone(),
        Arc::new(MacMarkdownWatcher),
    )))?;
    if let Some(quicklinks) = quicklinks {
        reg.register(Arc::new(QuicklinksModule::with_deps(
            Arc::new(SqliteQuicklinksRepository::new(quicklinks)),
            opener.clone(),
            pasteboard.clone(),
        )))?;
    } else {
        warn!("quicklinks store unavailable — Quicklinks module not registered");
    }
    if let Some(snippets) = snippets {
        reg.register(Arc::new(SnippetsModule::with_store(
            Arc::new(SqliteSnippetsRepository::new(snippets)),
            pasteboard.clone(),
            accessibility,
        )))?;
    } else {
        warn!("snippets store unavailable — Snippets module not registered");
    }
    reg.register(Arc::new(TodoModule::with_eventkit(Arc::new(MacEventKit))))?;
    reg.register(Arc::new(ProjectsModule::with_roots(
        project_roots,
        opener.clone(),
    )))?;
    reg.register(Arc::new(KillProcessModule::with_catalog(Arc::new(
        MacProcessCatalog,
    ))))?;
    reg.register(Arc::new(SecretsModule::with_deps(
        Arc::new(MacKeychain::luma_next()),
        pasteboard,
        clipboard_suppression,
    )))?;
    // Test/demo only — kept off unless explicitly enabled.
    reg.register(Arc::new(FakeEchoModule::new()))?;

    for (id, enabled) in &settings.enabled_modules {
        let _ = reg.set_enabled(id, *enabled);
    }
    if !settings
        .enabled_modules
        .get("luma.fake")
        .copied()
        .unwrap_or(false)
    {
        let _ = reg.set_enabled("luma.fake", false);
    }
    Ok(reg)
}

/// Load LumaNext settings + stores. Corrupt config is not replaced with defaults.
/// Individual store open failures are logged and skip that module — Apps/shell still start.
pub fn load_registry() -> Result<ModuleRegistry, RegistryError> {
    Ok(load_registry_with_settings()?.0)
}

/// Same as [`load_registry`], plus a settings repository for the engine.
pub fn load_registry_with_settings(
) -> Result<(ModuleRegistry, Arc<dyn SettingsRepository>), RegistryError> {
    let store = Arc::new(ConfigStore::luma_next_default()?);
    let settings = store.load_or_default()?;
    let clipboard = match ClipboardStore::luma_next_default() {
        Ok(s) => Some(Arc::new(s)),
        Err(err) => {
            warn!(%err, "failed to open clipboard store");
            None
        }
    };
    let quicklinks = match QuicklinksStore::luma_next_default() {
        Ok(s) => Some(Arc::new(s)),
        Err(err) => {
            warn!(%err, "failed to open quicklinks store");
            None
        }
    };
    let snippets = match SnippetsStore::luma_next_default() {
        Ok(s) => Some(Arc::new(s)),
        Err(err) => {
            warn!(%err, "failed to open snippets store");
            None
        }
    };
    let registry = registry_from_settings(&settings, clipboard, quicklinks, snippets)?;
    let settings_repo: Arc<dyn SettingsRepository> = Arc::new(TomlSettingsRepository::new(store));
    Ok((registry, settings_repo))
}
