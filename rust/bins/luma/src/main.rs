mod compose;

use clap::{Parser, Subcommand};
use compose::{load_registry, load_registry_with_settings};
use luma_application::{list_modules_json, run_action, run_doctor, run_query, Engine};
use luma_storage::{
    dry_run_legacy_dir, import_clipboard_fixture_with_ledger,
    import_notes_config_fixture_with_ledger, list_migrations, rollback_migration, ClipboardStore,
    ConfigStore,
};
use luma_tui::run_tui_with_engine;
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
    },
}

#[derive(Debug, Subcommand)]
enum ConfigCmd {
    Get {
        #[arg(long)]
        json: bool,
    },
    /// Patch LumaNext settings (CAS on settings_version).
    Set {
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
        json: bool,
    },
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

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
                    skipped_modules: load.skipped.into_iter().map(|s| (s.id, s.reason)).collect(),
                },
            ));
            run_tui_with_engine(engine).await?;
        }
        Some(Commands::Query { query, json }) => {
            let registry = load_registry().map_err(|e| anyhow::anyhow!("registry: {e}"))?;
            let (items, _events) = run_query(registry, &query)
                .await
                .map_err(anyhow::Error::msg)?;
            if json {
                let payload = serde_json::json!({
                    "query": query,
                    "results": items,
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                for item in items {
                    println!("{}\t{}", item.id, item.title);
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
                },
        }) => {
            let registry = load_registry().map_err(|e| anyhow::anyhow!("registry: {e}"))?;
            let (result, outcome) = run_action(
                registry,
                &query,
                result_id.as_deref(),
                &action_id,
                confirmation,
            )
            .await
            .map_err(anyhow::Error::msg)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "query": query,
                        "result": result,
                        "action_id": action_id,
                        "outcome": outcome,
                    }))?
                );
            } else {
                println!("{}\t{:?}", result.id, outcome);
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
        Some(Commands::Doctor { json }) => {
            let mut diag = match load_registry_with_settings() {
                Ok(load) => {
                    let mut d = run_doctor(load.registry, Some(load.settings))
                        .await
                        .map_err(anyhow::Error::msg)?;
                    d["skipped_modules"] = serde_json::json!(load
                        .skipped
                        .iter()
                        .map(|s| serde_json::json!({ "id": s.id, "reason": s.reason }))
                        .collect::<Vec<_>>());
                    if !load.skipped.is_empty() {
                        let mut remediation = d
                            .get("remediation")
                            .and_then(|v| v.as_array())
                            .cloned()
                            .unwrap_or_default();
                        for s in &load.skipped {
                            remediation.push(serde_json::json!(format!(
                                "Repair store for {} ({}), then restart Luma",
                                s.id, s.reason
                            )));
                        }
                        d["remediation"] = serde_json::Value::Array(remediation);
                    }
                    d
                }
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
                        diag["settings"] = serde_json::json!({
                            "configured": true,
                            "settings_version": settings.settings_version,
                            "notes_root_configured": settings.notes_root.is_some(),
                            "notes_root": settings.notes_root,
                            "projects_roots": settings.projects_roots,
                            "notes_exclude_patterns": settings.notes_exclude_patterns,
                            "clipboard_retention_days": settings.clipboard_retention_days,
                        });
                        if let Some(stores) = diag.get_mut("stores").and_then(|v| v.as_object_mut())
                        {
                            stores.insert("settings".into(), serde_json::json!("ok"));
                        }
                        diag["config_commands"] = serde_json::json!({
                            "notes_root": "luma config set --notes-root ~/Notes",
                            "projects_roots": "luma config set --projects-root ~/dev",
                            "notes_exclude": "luma config set --notes-exclude 'private/*'",
                            "clear_notes_excludes": "luma config set --clear-notes-excludes",
                        });
                        let ax_trusted = luma_platform_macos::MacAccessibility::probe_trusted();
                        let parent_pid = std::os::unix::process::parent_id();
                        let parent_name = std::process::Command::new("ps")
                            .args(["-p", &parent_pid.to_string(), "-o", "comm="])
                            .output()
                            .ok()
                            .filter(|o| o.status.success())
                            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                            .filter(|s| !s.is_empty());
                        let host = parent_name
                            .clone()
                            .unwrap_or_else(|| "the app that launched Luma".into());
                        let guidance = if ax_trusted {
                            format!("Accessibility trusted for current session (host: {host})")
                        } else {
                            format!(
                                "Grant Accessibility to {host} in System Settings → Privacy & Security → Accessibility"
                            )
                        };
                        diag["accessibility"] = serde_json::json!({
                            "trusted": ax_trusted,
                            "launch_executable": std::env::current_exe().ok().map(|p| p.display().to_string()),
                            "parent_pid": parent_pid,
                            "parent_name": parent_name,
                            "guidance": guidance,
                        });
                        // Legacy top-level mirror for scripts.
                        diag["ax_trusted"] = ax_trusted.into();
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
            if json {
                println!("{}", serde_json::to_string_pretty(&diag)?);
            } else {
                println!("{diag}");
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
            }
        }
        Some(Commands::Config {
            action:
                ConfigCmd::Set {
                    notes_root,
                    projects_root,
                    notes_exclude,
                    clear_notes_excludes,
                    enable_module,
                    disable_module,
                    json,
                },
        }) => {
            let store = ConfigStore::luma_next_default()?;
            let current = store.load_or_default()?;
            let expected = current.settings_version;
            let mut next = current.clone();
            if let Some(root) = notes_root {
                next.notes_root = if root.is_empty() { None } else { Some(root) };
            }
            if !projects_root.is_empty() {
                next.projects_roots = projects_root;
            }
            if clear_notes_excludes {
                next.notes_exclude_patterns.clear();
            }
            // Presence of any --notes-exclude flag replaces the list (including empty clear via
            // a dedicated empty value is not supported; omit the flag to leave unchanged).
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
            let saved = store.update_cas(expected, next)?;
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
