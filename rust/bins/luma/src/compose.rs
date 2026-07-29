//! Sole composition root helpers: wire settings, stores, and platform adapters into a registry.
//!
//! All modules are registered here; enable/disable is settings-driven. Disabled modules stay
//! listed in Settings but do not warm up or appear on the Hub.

use luma_application::{
    CapabilityPort, CommandRecipesRepository, CommandSpec, EmbeddedPtyPort, ModuleManifest,
    ModuleRegistry, PasteboardPort, RecallRepository, RegistryError as ModuleRegistryError,
    SearchMode, SettingsRepository, SqliteClipboardHistory, SqliteCommandRecipesRepository,
    SqliteDatabasePortalsRepository, SqliteQuicklinksRepository, SqliteRecallRepository,
    SqliteRecordsRepository, SqliteRenewalsRepository, SqliteSnippetsRepository,
    SqliteSshMetaRepository, SqliteTimersRepository, SqliteWordbookRepository,
    TomlSettingsRepository, UnavailableModule, WordbookRepository, WorkbenchMeta,
};
use luma_modules::{
    AppsModule, CalculatorModule, ClipboardModule, ClipboardSuppression, CommandRecipesModule,
    DatabasePortalsModule, DownloadsModule, FakeEchoModule, GitModule, PackagesModule,
    ProjectsModule, ProxyModule, QuicklinksModule, RecordsModule, RenewalsModule, RuntimeModule,
    ScreenOcrModule, SecretsModule, ShellHistoryModule, ShortcutsModule, SnippetsModule, SshModule,
    TimersModule, WindowsModule, WordbookModule,
};
use luma_platform_macos::{
    FilesystemAppsCatalog, MacAccessibility, MacBoundedUtf8FileReader, MacClock,
    MacDatabasePlatform, MacDownloads, MacEmbeddedPty, MacGitRepository, MacHomebrew, MacKeychain,
    MacMihomoProxyCore, MacNetworkProbe, MacOpenPath, MacPasteboard, MacProfileStore,
    MacProjectWorkspace, MacRecipeEnvironment, MacRuntimeInspector, MacScreenOcr, MacShellHistory,
    MacShortcuts, MacSpeech, MacSshConfig, MacSystemProxy, MacSystemSettings, MacWindowCatalog,
};
use luma_storage::{
    luma_next_support_dir, ClipboardStore, CommandRecipesMetaStore, ConfigError, ConfigStore,
    DatabasePortalsStore, LumaSettings, QuicklinksStore, RecallStore, RecordsStore, RenewalsStore,
    SnippetsStore, SshMetaStore, TimersStore, WordbookStore,
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
    pub command_recipes: Option<Arc<dyn CommandRecipesRepository>>,
    pub recall: Option<Arc<dyn RecallRepository>>,
    #[allow(dead_code)]
    pub skipped: Vec<SkippedModule>,
}

pub struct TuiPlatformAdapters {
    pub embedded_pty: Arc<dyn EmbeddedPtyPort>,
    pub pasteboard: Arc<dyn PasteboardPort>,
}

pub fn tui_platform_adapters() -> TuiPlatformAdapters {
    TuiPlatformAdapters {
        embedded_pty: Arc::new(MacEmbeddedPty::new()),
        pasteboard: Arc::new(MacPasteboard),
    }
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
/// A store failure becomes an in-place unavailable module rather than removing
/// its trigger and silently changing `/module` into a global search.
#[allow(clippy::too_many_arguments)]
pub fn registry_from_settings(
    settings: &LumaSettings,
    clipboard: Option<Arc<ClipboardStore>>,
    quicklinks: Option<Arc<QuicklinksStore>>,
    snippets: Option<Arc<SnippetsStore>>,
    wordbook: Option<Arc<WordbookStore>>,
    records: Option<Arc<RecordsStore>>,
    command_recipes_meta: Option<Arc<CommandRecipesMetaStore>>,
    timers: Option<Arc<TimersStore>>,
    renewals: Option<Arc<RenewalsStore>>,
    database_portals: Option<Arc<DatabasePortalsStore>>,
    recall: Option<Arc<RecallStore>>,
    support_dir: PathBuf,
) -> Result<(ModuleRegistry, Vec<SkippedModule>), ModuleRegistryError> {
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
    let accessibility = Arc::new(MacAccessibility::new());
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
    let integrate_clash_verge = proxy_core.uses_clash_verge_integration();
    let proxy_store = MacProfileStore::new(
        Arc::new(MacKeychain::private_references()),
        proxy_core.clone(),
        integrate_clash_verge,
    )
    .ok()
    .map(|store| Arc::new(store) as Arc<dyn luma_application::ProfileStorePort>);
    let mut proxy_module = ProxyModule::with_deps(
        proxy_core,
        Arc::new(MacSystemProxy::with_service(
            settings.proxy_network_service.clone(),
        )),
        pasteboard.clone(),
    )
    .with_network_probe(Arc::new(MacNetworkProbe));
    if let Some(proxy_store) = proxy_store {
        proxy_module = proxy_module.with_profile_store(proxy_store);
    }
    reg.register(Arc::new(proxy_module))?;
    reg.register(Arc::new(AppsModule::new(
        Arc::new(FilesystemAppsCatalog::system_default()),
        pasteboard.clone(),
    )))?;
    reg.register(Arc::new(CalculatorModule::with_deps(pasteboard.clone())))?;
    match MacDownloads::system_default() {
        Ok(downloads) => reg.register(Arc::new(DownloadsModule::with_deps(
            Arc::new(downloads),
            opener.clone(),
            pasteboard.clone(),
        )))?,
        Err(error) => {
            let reason = format!("Downloads adapter could not be configured: {error}");
            register_unavailable_store_module(
                &mut reg,
                &mut skipped,
                "luma.downloads",
                "Downloads Inbox",
                &["dl", "downloads"],
                "D",
                "/dl ",
                "/dl recent · large · old 30d · type image",
                false,
                DownloadsModule::command_specs(),
                &reason,
            )?;
        }
    }
    reg.register(Arc::new(PackagesModule::with_deps(
        Arc::new(MacHomebrew::system_default()),
        pasteboard.clone(),
    )))?;
    reg.register(Arc::new(ShortcutsModule::with_deps(
        Arc::new(MacShortcuts::system_default()),
        pasteboard.clone(),
    )))?;
    reg.register(Arc::new(
        ScreenOcrModule::with_deps(Arc::new(MacScreenOcr), pasteboard.clone())
            .with_system_settings(Arc::new(MacSystemSettings)),
    ))?;
    match MacShellHistory::system_default() {
        Ok(history) => reg.register(Arc::new(ShellHistoryModule::with_deps(
            Arc::new(history),
            pasteboard.clone(),
        )))?,
        Err(error) => {
            let reason = format!("Shell history adapter could not be configured: {error}");
            register_unavailable_store_module(
                &mut reg,
                &mut skipped,
                "luma.shell_history",
                "Shell Recall",
                &["hist", "history"],
                "H",
                "/hist ",
                "/hist recent · /hist <query>",
                false,
                ShellHistoryModule::command_specs(),
                &reason,
            )?;
        }
    }
    reg.register(Arc::new(WindowsModule::with_deps(
        window_catalog.clone(),
        Arc::new(MacSystemSettings),
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
        register_unavailable_store_module(
            &mut reg,
            &mut skipped,
            "luma.clipboard",
            "Clipboard",
            &["clip", "cb"],
            "C",
            "/clip ",
            "/clip · history · pin/unpin · paste needs AX",
            false,
            ClipboardModule::command_specs(),
            "Clipboard store could not be opened. Existing data was left untouched; close Luma, repair or restore the local store, then reopen.",
        )?;
    }
    if let Some(quicklinks) = quicklinks {
        reg.register(Arc::new(QuicklinksModule::with_deps(
            Arc::new(SqliteQuicklinksRepository::new(quicklinks)),
            opener.clone(),
            pasteboard.clone(),
        )))?;
    } else {
        register_unavailable_store_module(
            &mut reg,
            &mut skipped,
            "luma.quicklinks",
            "Quicklinks",
            &["ql", "quicklinks"],
            "Q",
            "/ql ",
            "/ql · /ql add <trigger> <url>",
            false,
            QuicklinksModule::command_specs(),
            "Quicklinks store could not be opened. Existing data was left untouched; close Luma, repair or restore the local store, then reopen.",
        )?;
    }
    if let Some(snippets) = snippets {
        reg.register(Arc::new(SnippetsModule::with_store(
            Arc::new(SqliteSnippetsRepository::new(snippets)),
            pasteboard.clone(),
            accessibility.clone(),
            window_catalog,
        )))?;
    } else {
        register_unavailable_store_module(
            &mut reg,
            &mut skipped,
            "luma.snippets",
            "Snippets",
            &["s", "snip"],
            "S",
            "/s ",
            "/s · /snip add <trigger> <body>",
            false,
            SnippetsModule::command_specs(),
            "Snippets store could not be opened. Existing data was left untouched; close Luma, repair or restore the local store, then reopen.",
        )?;
    }
    if let Some(wordbook) = wordbook {
        reg.register(Arc::new(WordbookModule::with_deps(
            Arc::new(SqliteWordbookRepository::new(wordbook)),
            pasteboard.clone(),
            Arc::new(MacSpeech),
            Arc::new(MacBoundedUtf8FileReader),
        )))?;
    } else {
        register_unavailable_store_module(
            &mut reg,
            &mut skipped,
            "luma.wordbook",
            "Wordbook",
            &["wb", "wordbook", "words"],
            "W",
            "/wb due",
            "/wb due · /wb new · /wb wrong · /wb status · /wb add TERM | meaning",
            false,
            WordbookModule::command_specs(),
            "Wordbook store could not be opened. Existing data was left untouched; close Luma, repair or restore the local store, then reopen.",
        )?;
    }
    if let Some(records) = records {
        reg.register(Arc::new(RecordsModule::with_deps(
            Arc::new(SqliteRecordsRepository::new(records)),
            records_root,
        )))?;
    } else {
        register_unavailable_store_module(
            &mut reg,
            &mut skipped,
            "luma.records",
            "Records",
            &["rec", "record"],
            "R",
            "/rec ",
            "/rec <query> · /rec browse 电影 · /rec add 电影 NAME | rating | note",
            true,
            RecordsModule::command_specs(),
            "Records store could not be opened. Existing data was left untouched; close Luma, repair or restore the local store, then reopen.",
        )?;
    }
    let git = Arc::new(MacGitRepository);
    let runtime = Arc::new(MacRuntimeInspector);
    let recipe_env = Arc::new(MacRecipeEnvironment::new());
    let command_recipes_repo = command_recipes_meta.as_ref().map(|meta| {
        Arc::new(SqliteCommandRecipesRepository::new(
            meta.clone(),
            support_dir.clone(),
        )) as Arc<dyn CommandRecipesRepository>
    });
    let recall_repo = recall.as_ref().map(|store| {
        Arc::new(SqliteRecallRepository::new(store.clone())) as Arc<dyn RecallRepository>
    });
    reg.register(Arc::new(
        ProjectsModule::with_deps(
            project_roots,
            settings.imported_projects.clone(),
            opener.clone(),
            Arc::new(MacProjectWorkspace),
        )
        .with_workbench_deps(
            git.clone(),
            runtime.clone(),
            recall_repo,
            command_recipes_repo.clone(),
            recipe_env.clone(),
        ),
    ))?;
    reg.register(Arc::new(GitModule::with_deps(
        settings.imported_projects.clone(),
        git,
        pasteboard.clone(),
    )))?;
    reg.register(Arc::new(RuntimeModule::with_deps(
        settings.imported_projects.clone(),
        runtime,
        pasteboard.clone(),
    )))?;
    if let Some(repo) = command_recipes_repo {
        reg.register(Arc::new(
            CommandRecipesModule::with_deps(repo, recipe_env, pasteboard.clone(), opener.clone())
                .with_projects(settings.imported_projects.clone()),
        ))?;
    } else {
        register_unavailable_store_module(
            &mut reg,
            &mut skipped,
            "luma.command_recipes",
            "Command Recipes",
            &["cmd", "recipe", "recipes"],
            "C",
            "/cmd ",
            "/cmd · /cmd test · r run · c copy · f favorite",
            false,
            CommandRecipesModule::command_specs(),
            "Command Recipes metadata store could not be opened. Existing data was left untouched; close Luma, repair or restore the local store, then reopen.",
        )?;
    }
    let ssh_meta = match SshMetaStore::luma_next_default() {
        Ok(s) => Some(Arc::new(s)),
        Err(err) => {
            warn!(%err, "failed to open ssh metadata store");
            None
        }
    };
    reg.register(Arc::new(
        SshModule::with_deps(
            Arc::new(MacSshConfig::system_default()),
            ssh_meta.map(|s| {
                Arc::new(SqliteSshMetaRepository::new(s))
                    as Arc<dyn luma_application::SshMetaRepository>
            }),
            pasteboard.clone(),
            Arc::new(MacClock),
        )
        .with_credentials(Arc::new(MacKeychain::ssh_passwords())),
    ))?;
    if let Some(timers) = timers {
        reg.register(Arc::new(TimersModule::with_deps(
            Arc::new(SqliteTimersRepository::new(timers)),
            Arc::new(MacClock),
            Arc::new(MacSpeech),
        )))?;
    } else {
        register_unavailable_store_module(
            &mut reg,
            &mut skipped,
            "luma.timers",
            "Timers",
            &["tm", "timer", "timers"],
            "T",
            "/tm ",
            "/tm · /tm pomo [min] [name] · /tm sw [name] · start/pause/resume",
            false,
            TimersModule::command_specs(),
            "Timers store could not be opened. Existing data was left untouched; close Luma, repair or restore the local store, then reopen.",
        )?;
    }
    if let Some(renewals) = renewals {
        reg.register(Arc::new(RenewalsModule::with_deps(
            Arc::new(SqliteRenewalsRepository::new(renewals)),
            Arc::new(MacClock),
        )))?;
    } else {
        register_unavailable_store_module(
            &mut reg,
            &mut skipped,
            "luma.renewals",
            "Renewals",
            &["renew", "renewals"],
            "R",
            "/renew ",
            "/renew · due · 30d · add · paid · cancel · delete · backup",
            false,
            RenewalsModule::command_specs(),
            "Renewals store could not be opened. Existing data was left untouched; close Luma, repair or restore the local store, then reopen.",
        )?;
    }
    if let Some(database_portals) = database_portals {
        reg.register(Arc::new(DatabasePortalsModule::with_deps(
            Arc::new(SqliteDatabasePortalsRepository::new(database_portals)),
            Arc::new(MacDatabasePlatform),
            opener.clone(),
            Arc::new(MacClock),
        )))?;
    } else {
        register_unavailable_store_module(
            &mut reg,
            &mut skipped,
            "luma.databases",
            "Database Portals",
            &["db", "database", "databases"],
            "D",
            "/db ",
            "/db add sqlite LABEL | PATH · /db add postgres LABEL | HOST | PORT | DB | USER",
            false,
            DatabasePortalsModule::command_specs(),
            "Database portal metadata store could not be opened. Existing data was left untouched; close Luma, repair or restore the local store, then reopen.",
        )?;
    }
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
    force_off_unless_explicitly_enabled(&mut reg, settings, "luma.fake");
    // Default-off until provisioned (ADR-0003). Missing key must not fall back to an
    // accidental manifest true — same force-off pattern as luma.fake.
    force_off_unless_explicitly_enabled(&mut reg, settings, "luma.secrets");
    force_off_unless_explicitly_enabled(&mut reg, settings, "luma.databases");
    for (id, reason) in reg.apply_capability_preflight(&ComposeCapabilities) {
        warn!("{id}: {reason} — module disabled");
        skipped.push(SkippedModule { id, reason });
    }
    Ok((reg, skipped))
}

fn force_off_unless_explicitly_enabled(
    registry: &mut ModuleRegistry,
    settings: &LumaSettings,
    module_id: &str,
) {
    if !settings
        .enabled_modules
        .get(module_id)
        .copied()
        .unwrap_or(false)
    {
        let _ = registry.set_enabled(module_id, false);
    }
}

#[allow(clippy::too_many_arguments)]
fn register_unavailable_store_module(
    reg: &mut ModuleRegistry,
    skipped: &mut Vec<SkippedModule>,
    id: &str,
    display_name: &str,
    triggers: &[&str],
    glyph: &str,
    suggested_query: &str,
    empty_hint: &str,
    supports_browse: bool,
    commands: Vec<CommandSpec>,
    reason: &str,
) -> Result<(), ModuleRegistryError> {
    warn!(module = id, "{reason}");
    reg.register(Arc::new(UnavailableModule::new(
        ModuleManifest {
            id: luma_domain::ModuleId::new(id),
            display_name: display_name.into(),
            triggers: triggers.iter().map(|trigger| (*trigger).into()).collect(),
            default_enabled: true,
            // Do not emit an unavailable row for every global search; a direct
            // slash command is the recovery surface for an unavailable store.
            search_mode: SearchMode::TargetedOnly,
            required_capabilities: vec![],
            workbench: WorkbenchMeta {
                glyph: Some(glyph.into()),
                suggested_query: Some(suggested_query.into()),
                empty_hint: Some(empty_hint.into()),
                supports_browse,
                commands,
            },
        },
        reason,
    )))?;
    skipped.push(SkippedModule {
        id: id.into(),
        reason: reason.into(),
    });
    Ok(())
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
    let support_dir = luma_next_support_dir().unwrap_or_else(|_| PathBuf::from("."));
    let command_recipes_meta = match CommandRecipesMetaStore::luma_next_default() {
        Ok(s) => Some(Arc::new(s)),
        Err(err) => {
            warn!(%err, "failed to open command recipes meta store");
            None
        }
    };
    let timers = match TimersStore::luma_next_default() {
        Ok(s) => Some(Arc::new(s)),
        Err(err) => {
            warn!(%err, "failed to open timers store");
            None
        }
    };
    let renewals = match RenewalsStore::luma_next_default() {
        Ok(s) => Some(Arc::new(s)),
        Err(err) => {
            warn!(%err, "failed to open renewals store");
            None
        }
    };
    let database_portals = match DatabasePortalsStore::luma_next_default() {
        Ok(s) => Some(Arc::new(s)),
        Err(err) => {
            warn!(%err, "failed to open database portals store");
            None
        }
    };
    let recall = match RecallStore::luma_next_default() {
        Ok(store) => Some(Arc::new(store)),
        Err(err) => {
            warn!(%err, "failed to open recall metadata store");
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
        command_recipes_meta.clone(),
        timers.clone(),
        renewals,
        database_portals,
        recall.clone(),
        support_dir.clone(),
    )?;
    let settings_repo: Arc<dyn SettingsRepository> = Arc::new(TomlSettingsRepository::new(store));
    let wordbook_repo: Option<Arc<dyn WordbookRepository>> =
        wordbook.map(|s| Arc::new(SqliteWordbookRepository::new(s)) as Arc<dyn WordbookRepository>);
    let command_recipes_repo: Option<Arc<dyn CommandRecipesRepository>> =
        command_recipes_meta.map(|meta| {
            Arc::new(SqliteCommandRecipesRepository::new(meta, support_dir))
                as Arc<dyn CommandRecipesRepository>
        });
    let recall_repo: Option<Arc<dyn RecallRepository>> = recall
        .map(|store| Arc::new(SqliteRecallRepository::new(store)) as Arc<dyn RecallRepository>);
    Ok(RegistryLoad {
        registry,
        settings: settings_repo,
        wordbook: wordbook_repo,
        command_recipes: command_recipes_repo,
        recall: recall_repo,
        skipped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn production_manifest_inventory_is_complete_and_command_driven() {
        let temp = tempfile::tempdir().unwrap();
        let settings = LumaSettings::default();
        let (registry, _) = registry_from_settings(
            &settings,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            temp.path().to_path_buf(),
        )
        .unwrap();
        let actual = registry
            .list_module_info()
            .into_iter()
            .filter(|module| module.id != "luma.fake")
            .map(|module| {
                assert!(!module.triggers.is_empty(), "{} has no trigger", module.id);
                for command in &module.commands {
                    assert!(
                        command.syntax.starts_with('/'),
                        "{} has non-slash syntax {}",
                        module.id,
                        command.syntax
                    );
                    assert!(
                        command.query.starts_with('/'),
                        "{} has non-slash query {}",
                        module.id,
                        command.query
                    );
                    if let Some(example) = &command.example {
                        assert!(
                            example.starts_with('/'),
                            "{} has non-slash example {example}",
                            module.id
                        );
                    }
                }
                (module.id, module.commands.len())
            })
            .collect::<BTreeMap<_, _>>();
        let expected = [
            ("luma.apps", 1),
            ("luma.calculator", 1),
            ("luma.clipboard", 5),
            ("luma.command_recipes", 3),
            ("luma.databases", 7),
            ("luma.downloads", 4),
            ("luma.git", 5),
            ("luma.ocr", 1),
            ("luma.packages", 6),
            ("luma.projects", 5),
            ("luma.proxy", 10),
            ("luma.quicklinks", 3),
            ("luma.records", 11),
            ("luma.renewals", 7),
            ("luma.runtime", 1),
            ("luma.secrets", 1),
            ("luma.shell_history", 1),
            ("luma.shortcuts", 3),
            ("luma.snippets", 4),
            ("luma.ssh", 5),
            ("luma.timers", 4),
            ("luma.windows", 1),
            ("luma.wordbook", 9),
        ]
        .into_iter()
        .map(|(id, count)| (id.to_string(), count))
        .collect::<BTreeMap<_, _>>();

        assert_eq!(actual, expected);
        assert_eq!(actual.values().sum::<usize>(), 98);
    }

    #[test]
    fn unavailable_renewals_store_keeps_id_and_triggers_registered() {
        let mut registry = ModuleRegistry::new();
        let mut skipped = Vec::new();
        register_unavailable_store_module(
            &mut registry,
            &mut skipped,
            "luma.renewals",
            "Renewals",
            &["renew", "renewals"],
            "R",
            "/renew ",
            "/renew add",
            false,
            RenewalsModule::command_specs(),
            "fixture failure",
        )
        .unwrap();
        assert!(registry.get("luma.renewals").is_some());
        assert_eq!(
            registry
                .resolve_trigger("renew")
                .unwrap()
                .manifest()
                .id
                .as_str(),
            "luma.renewals"
        );
        assert!(registry.all_triggers().contains(&"renewals".into()));
        assert!(registry
            .list_module_info()
            .iter()
            .find(|module| module.id == "luma.renewals")
            .is_some_and(|module| !module.commands.is_empty()));
        assert_eq!(skipped[0].id, "luma.renewals");
    }

    #[test]
    fn unavailable_database_store_keeps_triggers_but_stays_default_off() {
        let mut registry = ModuleRegistry::new();
        let mut skipped = Vec::new();
        register_unavailable_store_module(
            &mut registry,
            &mut skipped,
            "luma.databases",
            "Database Portals",
            &["db", "database", "databases"],
            "D",
            "/db ",
            "/db add sqlite",
            false,
            DatabasePortalsModule::command_specs(),
            "fixture failure",
        )
        .unwrap();
        let settings = LumaSettings::default();
        force_off_unless_explicitly_enabled(&mut registry, &settings, "luma.databases");
        assert!(registry.get("luma.databases").is_some());
        assert!(registry.all_triggers().contains(&"db".into()));
        assert!(!registry.is_enabled("luma.databases"));
        assert!(registry
            .list_module_info()
            .iter()
            .find(|module| module.id == "luma.databases")
            .is_some_and(|module| !module.commands.is_empty()));

        let mut enabled = LumaSettings::default();
        enabled
            .enabled_modules
            .insert("luma.databases".into(), true);
        registry.set_enabled("luma.databases", true);
        force_off_unless_explicitly_enabled(&mut registry, &enabled, "luma.databases");
        assert!(registry.is_enabled("luma.databases"));
    }
}
