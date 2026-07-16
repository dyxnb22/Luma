//! Sole composition root helpers: wire settings, stores, and platform adapters into a registry.
//!
//! All modules are registered here; enable/disable is settings-driven. Disabled modules stay
//! listed in Settings but do not warm up or appear on the Hub.

use luma_application::{
    CapabilityPort, ModuleRegistry, RegistryError as ModuleRegistryError, SettingsRepository,
    SqliteClipboardHistory, SqliteNotesIndex, SqliteQuicklinksRepository, SqliteRecordsRepository,
    SqliteSnippetsRepository, SqliteWordbookRepository, TomlSettingsRepository, WordbookRepository,
};
use luma_modules::{
    AppsModule, ClipboardModule, ClipboardSuppression, FakeEchoModule, NotesModule, NotesServices,
    ProjectsModule, ProxyModule, QuicklinksModule, RecordsModule, SecretsModule, SnippetsModule,
    WindowsModule, WordbookModule,
};
use luma_platform_macos::{
    FilesystemAppsCatalog, MacAccessibility, MacBoundedUtf8FileReader, MacClock, MacKeychain,
    MacMarkdownWatcher, MacMihomoProxyCore, MacNotesWorkspace, MacOpenPath, MacPasteboard,
    MacProfileStore, MacProjectWorkspace, MacSpeech, MacSystemProxy, MacWindowCatalog,
};
use luma_storage::{
    ClipboardStore, ConfigError, ConfigStore, LumaSettings, NotesIndexStore, NotesScanner,
    QuicklinksStore, RecordsStore, SnippetsStore, WordbookStore,
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

/// Module that could not be registered (store open failure, etc.).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkippedModule {
    pub id: String,
    pub reason: String,
}

/// Result of loading the composition root for the engine / TUI.
pub struct RegistryLoad {
    pub registry: ModuleRegistry,
    pub settings: Arc<dyn SettingsRepository>,
    pub wordbook: Option<Arc<dyn WordbookRepository>>,
    #[allow(dead_code)]
    pub skipped: Vec<SkippedModule>,
}

struct ComposeCapabilities;

impl CapabilityPort for ComposeCapabilities {
    fn has(&self, capability: &str) -> bool {
        match capability {
            "accessibility" => luma_platform_macos::MacAccessibility::probe_trusted(),
            // Service exists; empty label list is handled by the module.
            "keychain" => true,
            _ => true,
        }
    }
}

/// Build registry from settings + optionally opened stores.
/// Missing stores skip the corresponding module instead of failing the launcher.
pub fn registry_from_settings(
    settings: &LumaSettings,
    clipboard: Option<Arc<ClipboardStore>>,
    quicklinks: Option<Arc<QuicklinksStore>>,
    snippets: Option<Arc<SnippetsStore>>,
    wordbook: Option<Arc<WordbookStore>>,
    records: Option<Arc<RecordsStore>>,
    notes_index: Option<Arc<NotesScanner>>,
) -> Result<(ModuleRegistry, Vec<SkippedModule>), ModuleRegistryError> {
    let notes_root = settings.notes_root.as_ref().map(PathBuf::from);
    let records_root = settings
        .records_root
        .as_ref()
        .map(PathBuf::from)
        .or_else(default_records_root);
    let project_roots: Vec<PathBuf> = settings.projects_roots.iter().map(PathBuf::from).collect();
    let mut skipped = Vec::new();

    let opener = Arc::new(MacOpenPath);
    let pasteboard = Arc::new(MacPasteboard);
    let keychain = Arc::new(MacKeychain::luma_next());
    let accessibility = Arc::new(MacAccessibility);
    let clipboard_suppression = Arc::new(ClipboardSuppression::new());
    let window_catalog = Arc::new(MacWindowCatalog::new());
    if let Err(err) = window_catalog.snapshot_previous_frontmost_app_sync() {
        warn!(%err, "windows: previous-frontmost snapshot failed");
    }

    let mut reg = ModuleRegistry::new();
    let proxy_core = Arc::new(MacMihomoProxyCore::from_settings(
        settings,
        keychain.clone(),
    ));
    // Profile subscription references must remain private: they are not Secret-module labels
    // and must never become copyable UI entries.
    let proxy_store = MacProfileStore::new(
        Arc::new(MacKeychain::private_references()),
        proxy_core.clone(),
    )
    .ok()
    .map(|store| Arc::new(store) as Arc<dyn luma_application::ProfileStorePort>);
    let mut proxy_module = ProxyModule::with_deps(
        proxy_core,
        Arc::new(MacSystemProxy::with_service(
            settings.proxy_network_service.clone(),
        )),
        pasteboard.clone(),
    );
    if let Some(proxy_store) = proxy_store {
        proxy_module = proxy_module.with_profile_store(proxy_store);
    }
    reg.register(Arc::new(proxy_module))?;
    reg.register(Arc::new(AppsModule::new(
        Arc::new(FilesystemAppsCatalog::system_default()),
        pasteboard.clone(),
    )))?;
    reg.register(Arc::new(WindowsModule::with_catalog(
        window_catalog.clone(),
    )))?;
    if let Some(clipboard) = clipboard {
        reg.register(Arc::new(ClipboardModule::with_deps(
            Arc::new(SqliteClipboardHistory::new(clipboard)),
            pasteboard.clone(),
            accessibility.clone(),
            window_catalog.clone(),
            clipboard_suppression.clone(),
        )))?;
    } else {
        let reason = "clipboard store unavailable".into();
        warn!("{reason} — Clipboard module not registered");
        skipped.push(SkippedModule {
            id: "luma.clipboard".into(),
            reason,
        });
    }
    if let Some(scanner) = notes_index {
        reg.register(Arc::new(NotesModule::with_root(
            notes_root,
            opener.clone(),
            Arc::new(MacMarkdownWatcher),
            Arc::new(SqliteNotesIndex::with_exclude_patterns(
                scanner,
                settings.notes_exclude_patterns.clone(),
            )),
            pasteboard.clone(),
            NotesServices {
                clock: Arc::new(MacClock),
                workspace: Arc::new(MacNotesWorkspace),
            },
            settings.notes_exclude_patterns.clone(),
        )))?;
    } else {
        let reason = "notes index store unavailable".into();
        warn!("{reason} — Notes module not registered");
        skipped.push(SkippedModule {
            id: "luma.notes".into(),
            reason,
        });
    }
    if let Some(quicklinks) = quicklinks {
        reg.register(Arc::new(QuicklinksModule::with_deps(
            Arc::new(SqliteQuicklinksRepository::new(quicklinks)),
            opener.clone(),
            pasteboard.clone(),
        )))?;
    } else {
        let reason = "quicklinks store unavailable".into();
        warn!("{reason} — Quicklinks module not registered");
        skipped.push(SkippedModule {
            id: "luma.quicklinks".into(),
            reason,
        });
    }
    if let Some(snippets) = snippets {
        reg.register(Arc::new(SnippetsModule::with_store(
            Arc::new(SqliteSnippetsRepository::new(snippets)),
            pasteboard.clone(),
            accessibility.clone(),
            window_catalog,
        )))?;
    } else {
        let reason = "snippets store unavailable".into();
        warn!("{reason} — Snippets module not registered");
        skipped.push(SkippedModule {
            id: "luma.snippets".into(),
            reason,
        });
    }
    if let Some(wordbook) = wordbook {
        reg.register(Arc::new(WordbookModule::with_deps(
            Arc::new(SqliteWordbookRepository::new(wordbook)),
            pasteboard.clone(),
            Arc::new(MacSpeech),
            Arc::new(MacBoundedUtf8FileReader),
        )))?;
    } else {
        let reason = "wordbook store unavailable".into();
        warn!("{reason} — Wordbook module not registered");
        skipped.push(SkippedModule {
            id: "luma.wordbook".into(),
            reason,
        });
    }
    if let Some(records) = records {
        reg.register(Arc::new(RecordsModule::with_deps(
            Arc::new(SqliteRecordsRepository::new(records)),
            records_root,
        )))?;
    } else {
        let reason = "records store unavailable".into();
        warn!("{reason} — Records module not registered");
        skipped.push(SkippedModule {
            id: "luma.records".into(),
            reason,
        });
    }
    reg.register(Arc::new(ProjectsModule::with_settings(
        project_roots,
        settings.imported_projects.clone(),
        opener.clone(),
        Arc::new(MacProjectWorkspace),
    )))?;
    reg.register(Arc::new(SecretsModule::with_deps(
        keychain,
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
    for (id, reason) in reg.apply_capability_preflight(&ComposeCapabilities) {
        warn!("{id}: {reason} — module disabled");
        skipped.push(SkippedModule { id, reason });
    }
    Ok((reg, skipped))
}

fn default_records_root() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join("Documents/Notes/Records"))
}

/// Load LumaNext settings + stores. Corrupt config is not replaced with defaults.
/// Individual store open failures are logged and skip that module — Apps/shell still start.
pub fn load_registry() -> Result<ModuleRegistry, RegistryError> {
    Ok(load_registry_with_settings()?.registry)
}

/// Same as [`load_registry`], plus settings repository and skipped-module report.
pub fn load_registry_with_settings() -> Result<RegistryLoad, RegistryError> {
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
    let wordbook = match WordbookStore::luma_next_default() {
        Ok(s) => Some(Arc::new(s)),
        Err(err) => {
            warn!(%err, "failed to open wordbook store");
            None
        }
    };
    let records = match RecordsStore::luma_next_default() {
        Ok(s) => Some(Arc::new(s)),
        Err(err) => {
            warn!(%err, "failed to open records store");
            None
        }
    };
    let notes_index = match NotesIndexStore::luma_next_default() {
        Ok(store) => Some(Arc::new(NotesScanner::new(store))),
        Err(err) => {
            warn!(%err, "failed to open notes index");
            None
        }
    };
    let (registry, skipped) = registry_from_settings(
        &settings,
        clipboard.clone(),
        quicklinks.clone(),
        snippets.clone(),
        wordbook.clone(),
        records.clone(),
        notes_index.clone(),
    )?;
    let settings_repo: Arc<dyn SettingsRepository> = Arc::new(TomlSettingsRepository::new(store));
    let wordbook_repo: Option<Arc<dyn WordbookRepository>> =
        wordbook.map(|s| Arc::new(SqliteWordbookRepository::new(s)) as Arc<dyn WordbookRepository>);
    Ok(RegistryLoad {
        registry,
        settings: settings_repo,
        wordbook: wordbook_repo,
        skipped,
    })
}
