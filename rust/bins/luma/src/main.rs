mod cli_output;
mod compose;

use clap::{Parser, Subcommand};
use cli_output::{action_exit_code, format_doctor_summary};
use compose::{load_registry, load_registry_with_settings};
use luma_application::{
    list_modules_json, run_action, run_doctor_with_options, run_query, Engine, EngineOptions,
    KeychainPort,
};
use luma_storage::{
    dry_run_legacy_dir, import_clipboard_fixture_with_ledger,
    import_notes_config_fixture_with_ledger, list_migrations, rollback_migration, ClipboardStore,
    ConfigError, ConfigStore,
};
use luma_tui::run_tui_with_engine;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Parser)]
#[command(name = "luma", version, about = "Luma interactive CLI/TUI")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Run interactive TUI (default when no subcommand).
    Tui,
    /// Run a one-shot query through the application engine.
    Query {
        query: String,
        #[arg(long)]
        json: bool,
        /// Redact clip/snippet bodies in JSON (titles/subtitles replaced; safer for logs/gists).
        #[arg(long)]
        redact: bool,
    },
    /// Search then execute an action in the same engine session.
    Action {
        #[command(subcommand)]
        action: ActionCmd,
    },
    Modules {
        #[command(subcommand)]
        action: ModulesCmd,
    },
    Doctor {
        #[arg(long)]
        json: bool,
        /// Full diagnostic JSON instead of actionable summary.
        #[arg(long)]
        raw: bool,
    },
    Config {
        #[command(subcommand)]
        action: ConfigCmd,
    },
    /// Legacy / fixture importers (never modify source files).
    Migrate {
        #[command(subcommand)]
        action: MigrateCmd,
    },
    /// Manage Keychain-backed secrets (labels only in search).
    Secrets {
        #[command(subcommand)]
        action: SecretsCmd,
    },
}

#[derive(Debug, Subcommand)]
enum ModulesCmd {
    List {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum ActionCmd {
    Run {
        /// Query used to populate engine results before the action runs.
        #[arg(long)]
        query: String,
        /// Optional result id from that query; defaults to the highest-scoring result.
        #[arg(long)]
        result_id: Option<String>,
        #[arg(long)]
        action_id: String,
        #[arg(long)]
        confirmation: bool,
        #[arg(long)]
        json: bool,
        /// Redact clip/snippet bodies in JSON output (safer for logs/gists).
        #[arg(long)]
        redact: bool,
    },
}

#[derive(Debug, clap::Args)]
struct ConfigSetArgs {
    #[arg(long)]
    notes_root: Option<String>,
    #[arg(long)]
    projects_root: Vec<String>,
    /// Glob patterns relative to notes_root (repeatable). Replaces the full list when set.
    #[arg(long)]
    notes_exclude: Vec<String>,
    /// Clear all notes_exclude_patterns.
    #[arg(long)]
    clear_notes_excludes: bool,
    #[arg(long)]
    enable_module: Vec<String>,
    #[arg(long)]
    disable_module: Vec<String>,
    #[arg(long)]
    clipboard_retention_days: Option<u32>,
    /// Lock Secrets vault after N idle seconds (`0` disables).
    #[arg(long)]
    secrets_idle_lock_secs: Option<u32>,
    /// Max Hub window rows (clamped 5–50).
    #[arg(long)]
    hub_windows_max: Option<u32>,
    /// CAS guard: fail unless settings.toml is at this settings_version.
    #[arg(long = "expected-version")]
    expected_version: Option<u64>,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Subcommand)]
enum ConfigCmd {
    Get {
        #[arg(long)]
        json: bool,
    },
    /// Patch LumaNext settings (compare-and-swap on settings_version).
    Set(ConfigSetArgs),
}

#[derive(Debug, Subcommand)]
enum SecretsCmd {
    /// Store a secret: reads value from stdin (not argv). Updates Keychain + label sidecar.
    Set { account: String },
}

#[derive(Debug, Subcommand)]
enum MigrateCmd {
    DryRun {
        /// Legacy Application Support path (default: ~/Library/Application Support/Luma)
        #[arg(long)]
        legacy: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Import a desensitized clipboard fixture JSON into LumaNext SQLite.
    ClipboardFixture {
        /// Path to fixture JSON (default: rust/fixtures/legacy/clipboard-history.sample.json)
        #[arg(long)]
        path: Option<PathBuf>,
        /// Do not write the store
        #[arg(long, default_value_t = true)]
        dry_run: bool,
        /// Actually write (overrides dry_run)
        #[arg(long)]
        commit: bool,
        #[arg(long)]
        json: bool,
    },
    /// Import clipboard-history.json from an explicit path (source read-only; writes LumaNext only).
    Clipboard {
        #[arg(long)]
        legacy: PathBuf,
        #[arg(long, default_value_t = true)]
        dry_run: bool,
        #[arg(long)]
        commit: bool,
        #[arg(long)]
        json: bool,
    },
    /// Import notes root from notes.json fixture/legacy file into LumaNext settings.
    NotesConfig {
        #[arg(long)]
        legacy: PathBuf,
        #[arg(long, default_value_t = true)]
        dry_run: bool,
        #[arg(long)]
        commit: bool,
        #[arg(long)]
        json: bool,
    },
    /// List persisted migration ledger entries under LumaNext.
    List {
        #[arg(long)]
        json: bool,
    },
    /// Roll back a committed migration by id (restores LumaNext snapshot; never touches legacy).
    Rollback {
        #[arg(long)]
        migration_id: String,
        #[arg(long)]
        json: bool,
    },
}

fn init_tracing() {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::Layer;
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));
    let stderr = tracing_subscriber::fmt::layer().with_writer(std::io::stderr);
    let mut layers = vec![stderr.boxed()];
    if let Ok(dir) = luma_storage::luma_next_logs_dir() {
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("luma.log");
        if let Ok(file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let file_layer =
                tracing_subscriber::fmt::layer().with_writer(std::sync::Mutex::new(file));
            layers.push(file_layer.boxed());
        }
    }
    tracing_subscriber::registry()
        .with(filter)
        .with(layers)
        .init();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let cli = Cli::parse();
    match cli.command {
        None | Some(Commands::Tui) => {
            let load =
                load_registry_with_settings().map_err(|e| anyhow::anyhow!("registry: {e}"))?;
            let diagnostics = luma_application::FsDiagnosticsSink::luma_next_default()
                .ok()
                .map(|s| Arc::new(s) as Arc<dyn luma_application::DiagnosticsSink>);
            let engine: Arc<dyn luma_application::EnginePort> = Arc::new(Engine::with_options(
                load.registry,
                luma_application::EngineOptions {
                    settings: Some(load.settings),
                    diagnostics,
                    storage_probe: Some(load.storage_probe),
                    platform_probe: Some(load.platform_probe),
                    skipped_modules: load.skipped.into_iter().map(|s| (s.id, s.reason)).collect(),
                },
            ));
            run_tui_with_engine(engine).await?;
        }
        Some(Commands::Query {
            query,
            json,
            redact,
        }) => {
            let load =
                load_registry_with_settings().map_err(|e| anyhow::anyhow!("registry: {e}"))?;
            let (items, _events) = run_query(load.registry, &query, Some(load.settings))
                .await
                .map_err(anyhow::Error::msg)?;
            if json {
                let results: Vec<_> = if redact {
                    items
                        .into_iter()
                        .map(|mut item| {
                            let sensitive = item.module_id.as_str() == "luma.clipboard"
                                || item.module_id.as_str() == "luma.snippets";
                            if sensitive {
                                item.title = "[redacted]".into();
                                item.subtitle = Some("[redacted]".into());
                            }
                            item
                        })
                        .collect()
                } else {
                    items
                };
                let payload = serde_json::json!({
                    "query": query,
                    "redacted": redact,
                    "results": results,
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                for item in items {
                    if redact
                        && (item.module_id.as_str() == "luma.clipboard"
                            || item.module_id.as_str() == "luma.snippets")
                    {
                        println!("{}\t[redacted]", item.id);
                    } else {
                        println!("{}\t{}", item.id, item.title);
                    }
                }
            }
        }
        Some(Commands::Action {
            action:
                ActionCmd::Run {
                    query,
                    result_id,
                    action_id,
                    confirmation,
                    json,
                    redact,
                },
        }) => {
            let load =
                load_registry_with_settings().map_err(|e| anyhow::anyhow!("registry: {e}"))?;
            let (result, outcome) = run_action(
                load.registry,
                &query,
                result_id.as_deref(),
                &action_id,
                confirmation,
                Some(load.settings),
            )
            .await
            .map_err(anyhow::Error::msg)?;
            if json {
                let result = if redact {
                    let mut result = result;
                    let sensitive = result.module_id.as_str() == "luma.clipboard"
                        || result.module_id.as_str() == "luma.snippets";
                    if sensitive {
                        result.title = "[redacted]".into();
                        result.subtitle = Some("[redacted]".into());
                    }
                    result
                } else {
                    result
                };
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "query": query,
                        "redacted": redact,
                        "result": result,
                        "action_id": action_id,
                        "outcome": outcome,
                    }))?
                );
            } else {
                println!("{}\t{:?}", result.id, outcome);
            }
            let code = action_exit_code(&outcome);
            if code != 0 {
                std::process::exit(code);
            }
        }
        Some(Commands::Modules {
            action: ModulesCmd::List { json },
        }) => {
            let registry = load_registry().map_err(|e| anyhow::anyhow!("registry: {e}"))?;
            let payload = list_modules_json(&registry).await;
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else if let Some(arr) = payload["modules"].as_array() {
                for m in arr {
                    println!(
                        "{}\t{}\t{}",
                        m["id"].as_str().unwrap_or(""),
                        if m["enabled"].as_bool().unwrap_or(false) {
                            "on"
                        } else {
                            "off"
                        },
                        m["display_name"].as_str().unwrap_or("")
                    );
                }
            }
        }
        Some(Commands::Doctor { json, raw }) => {
            let mut diag = match load_registry_with_settings() {
                Ok(load) => run_doctor_with_options(
                    load.registry,
                    EngineOptions {
                        settings: Some(load.settings),
                        diagnostics: None,
                        storage_probe: Some(load.storage_probe),
                        platform_probe: Some(load.platform_probe),
                        skipped_modules: load
                            .skipped
                            .into_iter()
                            .map(|s| (s.id, s.reason))
                            .collect(),
                    },
                )
                .await
                .map_err(anyhow::Error::msg)?,
                Err(err) => serde_json::json!({
                    "ok": false,
                    "registry_error": err.to_string(),
                }),
            };
            if let Ok(store) = ConfigStore::luma_next_default() {
                match store.load_or_default() {
                    Ok(settings) => {
                        // Keep top-level mirrors for existing CLI consumers; nested
                        // `settings` already comes from Engine when SettingsRepository is wired.
                        diag["settings_version"] = settings.settings_version.into();
                        diag["notes_root_configured"] = settings.notes_root.is_some().into();
                        diag["notes_root"] = settings.notes_root.clone().into();
                        diag["projects_roots"] = settings.projects_roots.clone().into();
                        diag["notes_exclude_patterns"] =
                            settings.notes_exclude_patterns.clone().into();
                        if let Some(settings_obj) =
                            diag.get_mut("settings").and_then(|v| v.as_object_mut())
                        {
                            settings_obj.insert("configured".into(), true.into());
                            settings_obj.insert(
                                "clipboard_retention_days".into(),
                                settings.clipboard_retention_days.into(),
                            );
                            settings_obj.insert(
                                "secrets_idle_lock_secs".into(),
                                settings.secrets_idle_lock_secs.into(),
                            );
                            settings_obj
                                .insert("hub_windows_max".into(), settings.hub_windows_max.into());
                        }
                        if let Some(stores) = diag.get_mut("stores").and_then(|v| v.as_object_mut())
                        {
                            stores.insert("settings".into(), serde_json::json!("ok"));
                        }
                        diag["config_commands"] = serde_json::json!({
                            "notes_root": "luma config set --notes-root ~/Notes",
                            "projects_roots": "luma config set --projects-root ~/dev",
                            "notes_exclude": "luma config set --notes-exclude 'private/*'",
                            "clear_notes_excludes": "luma config set --clear-notes-excludes",
                            "secrets_idle_lock_secs": "luma config set --secrets-idle-lock-secs 300",
                            "hub_windows_max": "luma config set --hub-windows-max 15",
                        });
                        if settings.notes_root.is_none() {
                            let mut remediation = diag
                                .get("remediation")
                                .and_then(|v| v.as_array())
                                .cloned()
                                .unwrap_or_default();
                            remediation.insert(
                                0,
                                serde_json::json!("Notes: luma config set --notes-root ~/Notes"),
                            );
                            diag["remediation"] = serde_json::Value::Array(remediation);
                        }
                    }
                    Err(err) => {
                        diag["config_error"] = err.to_string().into();
                    }
                }
            }
            if json || raw {
                println!("{}", serde_json::to_string_pretty(&diag)?);
            } else {
                let summary = format_doctor_summary(&diag);
                if summary.trim().is_empty() {
                    println!("{}", serde_json::to_string_pretty(&diag)?);
                } else {
                    println!("{summary}");
                }
            }
        }
        Some(Commands::Config {
            action: ConfigCmd::Get { json },
        }) => {
            let store = ConfigStore::luma_next_default()?;
            let settings = store.load_or_default()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&settings)?);
            } else {
                println!(
                    "settings_version={} notes_root={:?} projects_roots={:?}",
                    settings.settings_version, settings.notes_root, settings.projects_roots
                );
                println!(
                    "notes_exclude_patterns={:?}",
                    settings.notes_exclude_patterns
                );
                println!("enabled_modules={:?}", settings.enabled_modules);
                println!(
                    "clipboard_retention_days={}",
                    settings.clipboard_retention_days
                );
                println!("secrets_idle_lock_secs={}", settings.secrets_idle_lock_secs);
                println!("hub_windows_max={}", settings.hub_windows_max);
            }
        }
        Some(Commands::Config {
            action: ConfigCmd::Set(args),
        }) => {
            let ConfigSetArgs {
                notes_root,
                projects_root,
                notes_exclude,
                clear_notes_excludes,
                enable_module,
                disable_module,
                clipboard_retention_days,
                secrets_idle_lock_secs,
                hub_windows_max,
                expected_version,
                json,
            } = args;
            let store = ConfigStore::luma_next_default()?;
            let saved = match store.mutate_settings(expected_version, |next| {
                if let Some(root) = notes_root {
                    next.notes_root = if root.is_empty() { None } else { Some(root) };
                }
                if !projects_root.is_empty() {
                    next.projects_roots = projects_root;
                }
                if clear_notes_excludes {
                    next.notes_exclude_patterns.clear();
                }
                if !notes_exclude.is_empty() {
                    next.notes_exclude_patterns = notes_exclude
                        .into_iter()
                        .filter(|p| !p.is_empty())
                        .collect();
                }
                for id in enable_module {
                    next.enabled_modules.insert(id, true);
                }
                for id in disable_module {
                    next.enabled_modules.insert(id, false);
                }
                if let Some(days) = clipboard_retention_days {
                    next.clipboard_retention_days = days;
                }
                if let Some(secs) = secrets_idle_lock_secs {
                    next.secrets_idle_lock_secs = secs;
                }
                if let Some(max) = hub_windows_max {
                    next.hub_windows_max = max.clamp(5, 50);
                }
            }) {
                Ok(s) => s,
                Err(ConfigError::VersionConflict { expected, found }) => {
                    anyhow::bail!("version conflict: expected {expected}, found {found}");
                }
                Err(ConfigError::LockTimeout) => {
                    anyhow::bail!("settings lock timeout — another Luma instance may be saving");
                }
                Err(err) => return Err(err.into()),
            };
            if json {
                println!("{}", serde_json::to_string_pretty(&saved)?);
            } else {
                println!(
                    "updated settings_version={} notes_root={:?} notes_exclude={:?}",
                    saved.settings_version, saved.notes_root, saved.notes_exclude_patterns
                );
            }
        }
        Some(Commands::Migrate {
            action: MigrateCmd::DryRun { legacy, json },
        }) => {
            let path = legacy.unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_default()
                    .join("Library/Application Support/Luma")
            });
            let report = dry_run_legacy_dir(path);
            if json {
                println!("{}", serde_json::to_string_pretty(&report.ledger)?);
            } else {
                for note in report.notes {
                    println!("{note}");
                }
            }
        }
        Some(Commands::Migrate {
            action:
                MigrateCmd::ClipboardFixture {
                    path,
                    dry_run,
                    commit,
                    json,
                },
        }) => {
            let fixture = path.unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../fixtures/legacy/clipboard-history.sample.json")
            });
            let store = ClipboardStore::luma_next_default()?;
            let do_dry = if commit { false } else { dry_run };
            let report = import_clipboard_fixture_with_ledger(&fixture, &store, do_dry, true)?;
            print_import_report(report, json)?;
        }
        Some(Commands::Migrate {
            action:
                MigrateCmd::Clipboard {
                    legacy,
                    dry_run,
                    commit,
                    json,
                },
        }) => {
            let path = if legacy.is_dir() {
                legacy.join("clipboard-history.json")
            } else {
                legacy
            };
            let store = ClipboardStore::luma_next_default()?;
            let do_dry = if commit { false } else { dry_run };
            let report = import_clipboard_fixture_with_ledger(&path, &store, do_dry, true)?;
            print_import_report(report, json)?;
        }
        Some(Commands::Migrate {
            action:
                MigrateCmd::NotesConfig {
                    legacy,
                    dry_run,
                    commit,
                    json,
                },
        }) => {
            let path = if legacy.is_dir() {
                legacy.join("notes.json")
            } else {
                legacy
            };
            let _ = ConfigStore::luma_next_default()?;
            let settings_path = luma_storage::luma_next_support_dir()?.join("settings.toml");
            let do_dry = if commit { false } else { dry_run };
            let report = import_notes_config_fixture_with_ledger(&path, &settings_path, do_dry)?;
            print_import_report(report, json)?;
        }
        Some(Commands::Migrate {
            action: MigrateCmd::List { json },
        }) => {
            let entries = list_migrations()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&entries)?);
            } else {
                for e in entries {
                    println!(
                        "{}\t{:?}\t{:?}\timported={} dry_run={}",
                        e.migration_id, e.kind, e.status, e.imported, e.dry_run
                    );
                }
            }
        }
        Some(Commands::Migrate {
            action: MigrateCmd::Rollback { migration_id, json },
        }) => {
            let support = luma_storage::luma_next_support_dir()?;
            let settings = support.join("settings.toml");
            let clip = support.join("clipboard.sqlite");
            let record = rollback_migration(
                &migration_id,
                &[
                    ("settings.toml", settings.as_path()),
                    ("clipboard.sqlite", clip.as_path()),
                ],
            )?;
            if json {
                println!("{}", serde_json::to_string_pretty(&record)?);
            } else {
                println!(
                    "rolled_back={} status={:?}",
                    record.migration_id, record.status
                );
            }
        }
        Some(Commands::Secrets {
            action: SecretsCmd::Set { account },
        }) => {
            if account.trim().is_empty() {
                anyhow::bail!("account must not be empty");
            }
            let mut value = String::new();
            std::io::stdin().read_to_string(&mut value)?;
            let value = value.trim_end_matches(['\n', '\r']);
            if value.is_empty() {
                anyhow::bail!("empty secret value on stdin");
            }
            let keychain = luma_platform_macos::MacKeychain::luma_next();
            keychain
                .set_password(account.trim(), value)
                .await
                .map_err(|e| anyhow::anyhow!("secrets set: {e}"))?;
            println!("stored label {account}");
        }
    }
    Ok(())
}

fn print_import_report(report: luma_storage::ImportReport, json: bool) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "imported={} skipped={} errors={} dry_run={}",
            report.ledger.imported,
            report.ledger.skipped,
            report.ledger.errors,
            report.ledger.dry_run
        );
        for note in report.notes {
            println!("{note}");
        }
    }
    Ok(())
}
