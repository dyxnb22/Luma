//! Sole composition root helpers: wire settings, stores, and platform adapters into a registry.
//!
//! All modules are registered here; enable/disable is settings-driven. Disabled modules stay
//! listed in Settings but do not warm up or appear on the Hub.

use luma_application::{
    CommandRecipesRepository, CommandSpec, ModuleManifest, ModuleRegistry, RecallRepository,
    RegistryError as ModuleRegistryError, SearchMode, SettingsRepository, SqliteClipboardHistory,
    SqliteCommandRecipesRepository, SqliteRecallRepository, SqliteRecordsRepository,
    SqliteWordbookRepository, TomlSettingsRepository, UnavailableModule, WordbookRepository,
    WorkbenchMeta,
};
use luma_modules::{
    AppsModule, ClipboardModule, ClipboardSuppression, CommandRecipesModule, FakeEchoModule,
    GitModule, ProjectsModule, RecordsModule, RuntimeModule, WindowsModule, WordbookModule,
};
use luma_platform_macos::{
    FilesystemAppsCatalog, MacAccessibility, MacBoundedUtf8FileReader, MacGitRepository,
    MacOpenPath, MacPasteboard, MacProjectWorkspace, MacRecipeEnvironment, MacRuntimeInspector,
    MacSpeech, MacSystemSettings, MacWindowCatalog,
};
use luma_storage::{
    luma_next_support_dir, ClipboardStore, CommandRecipesMetaStore, ConfigError, ConfigStore,
    LumaSettings, RecallStore, RecordsStore, WordbookStore,
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

/// Build registry from settings + optionally opened stores.
/// A store failure becomes an in-place unavailable module rather than removing
/// its trigger and silently changing `/module` into a global search.
pub fn registry_from_settings(
    settings: &LumaSettings,
    clipboard: Option<Arc<ClipboardStore>>,
    wordbook: Option<Arc<WordbookStore>>,
    records: Option<Arc<RecordsStore>>,
    command_recipes_meta: Option<Arc<CommandRecipesMetaStore>>,
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
    let accessibility = Arc::new(MacAccessibility::new());
    let clipboard_suppression = Arc::new(ClipboardSuppression::new());
    let window_catalog = Arc::new(MacWindowCatalog::new());
    if let Err(err) = window_catalog.snapshot_previous_frontmost_app_sync() {
        warn!(%err, "windows: previous-frontmost snapshot failed");
    }

    let mut reg = ModuleRegistry::new();
    reg.register(Arc::new(AppsModule::new(
        Arc::new(FilesystemAppsCatalog::system_default()),
        pasteboard.clone(),
    )))?;
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
    // Test/demo only — kept off unless explicitly enabled.
    reg.register(Arc::new(FakeEchoModule::new()))?;

    for (id, enabled) in &settings.enabled_modules {
        let _ = reg.set_enabled(id, *enabled);
    }
    Ok((reg, skipped))
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
/// Individual store open failures are logged and skip that module — Apps and the TUI still start.
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
        wordbook.clone(),
        records.clone(),
        command_recipes_meta.clone(),
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
            ("luma.clipboard", 5),
            ("luma.command_recipes", 3),
            ("luma.git", 5),
            ("luma.projects", 5),
            ("luma.records", 11),
            ("luma.runtime", 1),
            ("luma.windows", 1),
            ("luma.wordbook", 9),
        ]
        .into_iter()
        .map(|(id, count)| (id.to_string(), count))
        .collect::<BTreeMap<_, _>>();

        assert_eq!(actual, expected);
        assert_eq!(actual.values().sum::<usize>(), 41);
    }
}
