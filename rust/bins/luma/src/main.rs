mod cli_output;
mod compose;
mod ssh_cli;

use clap::{Parser, Subcommand};
use cli_output::action_exit_code;
use compose::{load_registry, load_registry_with_settings};
use luma_application::{
    list_modules_json, run_action, run_query, Engine, KeychainPort, RecordsRepository,
    SqliteRecordsRepository,
};
use luma_storage::{
    dry_run_legacy_dir, get_migration, import_clipboard_fixture_with_ledger,
    import_notes_config_fixture_with_ledger, import_records_with_ledger, list_migrations,
    preview_import_from_dir, record_dry_run, rollback_migration, ClipboardStore, ConfigError,
    ConfigStore, MigrationKind, RecordsStore, WordbookStore,
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
#[allow(clippy::large_enum_variant)]
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
    /// Wordbook vocab / import from WordPet.
    Wordbook {
        #[command(subcommand)]
        action: WordbookCmd,
    },
    /// Personal media/content records (movies, games, …).
    Record {
        #[command(subcommand)]
        action: RecordCmd,
    },
    /// SSH host connections from ~/.ssh/config.
    Ssh {
        #[command(subcommand)]
        action: SshCmd,
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
    records_root: Option<String>,
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
    /// Mihomo Unix controller socket path (loopback-only adapter).
    #[arg(long)]
    proxy_controller_unix_socket: Option<String>,
    /// Mihomo loopback controller address, for example 127.0.0.1:9097.
    #[arg(long)]
    proxy_controller_address: Option<String>,
    /// Luma Keychain account containing the Mihomo controller secret.
    #[arg(long)]
    proxy_controller_secret_account: Option<String>,
    /// Explicit macOS Network Service name for system proxy changes.
    #[arg(long)]
    proxy_network_service: Option<String>,
    /// Import a project directory (canonical path; repeatable).
    #[arg(long)]
    import_project: Vec<String>,
    /// Remove an imported project by name or path (config only; repeatable).
    #[arg(long)]
    remove_project: Vec<String>,
    /// CAS guard: fail unless settings.toml is at this settings_version.
    #[arg(long = "expected-version")]
    expected_version: Option<u64>,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
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
enum WordbookCmd {
    /// Import a WordPet/WordBot sqlite database (preserves review progress).
    ImportWordpet {
        #[arg(long = "from")]
        from: PathBuf,
        /// Write into LumaNext wordbook.sqlite (default is dry-run).
        #[arg(long)]
        commit: bool,
        #[arg(long)]
        json: bool,
    },
    /// Copy wordbook.sqlite into LumaNext/backups/.
    Backup {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum RecordCmd {
    /// Show DB stats.
    Status {
        #[arg(long)]
        json: bool,
    },
    /// List categories or records in a category.
    Browse {
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Add a record.
    Add {
        category: String,
        name: String,
        #[arg(long)]
        rating: Option<i64>,
        #[arg(long)]
        note: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Set rating (1-10) or omit with --clear.
    Rate {
        id: i64,
        score: Option<i64>,
        #[arg(long)]
        clear: bool,
        #[arg(long)]
        json: bool,
    },
    /// Update note text.
    Note {
        id: i64,
        text: String,
        #[arg(long)]
        json: bool,
    },
    /// Remove a record.
    Remove {
        id: i64,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        json: bool,
    },
    /// Import Markdown tables from a directory (default dry-run).
    Import {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
    /// Import status and last migration summary.
    ImportStatus {
        #[arg(long)]
        json: bool,
    },
    /// Copy records.sqlite into LumaNext/backups/.
    Backup {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum SshCmd {
    /// List configured SSH hosts.
    List {
        #[arg(long)]
        json: bool,
    },
    /// Connect via ssh in the current terminal.
    Connect { alias: String },
    /// Open SFTP in the current terminal.
    Sftp { alias: String },
    /// Mark a host as favorite.
    Favorite { alias: String },
    /// Remove favorite from a host.
    Unfavorite { alias: String },
    /// Set a local display name for a host.
    Rename { alias: String, name: String },
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
            let engine: Arc<dyn luma_application::EnginePort> = Arc::new(Engine::with_options(
                load.registry,
                luma_application::EngineOptions {
                    settings: Some(load.settings),
                    wordbook: load.wordbook,
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
                if settings.imported_projects.is_empty() {
                    println!("imported_projects=(none)");
                } else {
                    for p in &settings.imported_projects {
                        let name = p.name.as_deref().unwrap_or("(unnamed)");
                        println!("imported_project={name}\t{}", p.path);
                    }
                }
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
                println!(
                    "proxy_controller_unix_socket={:?}",
                    settings.proxy_controller_unix_socket
                );
                println!(
                    "proxy_controller_address={:?}",
                    settings.proxy_controller_address
                );
                println!(
                    "proxy_controller_secret_account={:?}",
                    settings.proxy_controller_secret_account
                );
                println!("proxy_network_service={:?}", settings.proxy_network_service);
            }
        }
        Some(Commands::Config {
            action: ConfigCmd::Set(args),
        }) => {
            let ConfigSetArgs {
                notes_root,
                records_root,
                projects_root,
                notes_exclude,
                clear_notes_excludes,
                enable_module,
                disable_module,
                clipboard_retention_days,
                secrets_idle_lock_secs,
                hub_windows_max,
                proxy_controller_unix_socket,
                proxy_controller_address,
                proxy_controller_secret_account,
                proxy_network_service,
                import_project,
                remove_project,
                expected_version,
                json,
            } = args;
            let store = ConfigStore::luma_next_default()?;
            let saved = match store.try_mutate_settings(expected_version, |next| {
                if let Some(root) = notes_root {
                    next.notes_root = if root.is_empty() { None } else { Some(root) };
                }
                if let Some(root) = records_root {
                    next.records_root = if root.is_empty() { None } else { Some(root) };
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
                if let Some(path) = proxy_controller_unix_socket {
                    next.proxy_controller_unix_socket =
                        if path.is_empty() { None } else { Some(path) };
                }
                if let Some(address) = proxy_controller_address {
                    next.proxy_controller_address = if address.is_empty() {
                        None
                    } else {
                        Some(address)
                    };
                }
                if let Some(account) = proxy_controller_secret_account {
                    next.proxy_controller_secret_account = if account.is_empty() {
                        None
                    } else {
                        Some(account)
                    };
                }
                if let Some(service) = proxy_network_service {
                    next.proxy_network_service = if service.is_empty() {
                        None
                    } else {
                        Some(service)
                    };
                }
                for path in &import_project {
                    next.import_project_path(std::path::Path::new(path))
                        .map_err(|err| err.to_string())?;
                }
                for name in &remove_project {
                    next.remove_imported_project(name)
                        .map_err(|err| err.to_string())?;
                }
                Ok(())
            }) {
                Ok(s) => s,
                Err(ConfigError::VersionConflict { expected, found }) => {
                    anyhow::bail!("version conflict: expected {expected}, found {found}");
                }
                Err(ConfigError::LockTimeout) => {
                    anyhow::bail!("settings lock timeout — another Luma instance may be saving");
                }
                Err(ConfigError::Mutation(message)) => anyhow::bail!("{message}"),
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
            let records = support.join("records.sqlite");
            let migration = get_migration(&migration_id)?;
            let targets: Vec<(&str, &std::path::Path)> = match migration.kind {
                MigrationKind::Clipboard => vec![("clipboard.sqlite", clip.as_path())],
                MigrationKind::NotesConfig => vec![("settings.toml", settings.as_path())],
                MigrationKind::Records => vec![("records.sqlite", records.as_path())],
                MigrationKind::LegacyDryRun => vec![],
            };
            let record = rollback_migration(&migration_id, &targets)?;
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
        Some(Commands::Wordbook {
            action: WordbookCmd::ImportWordpet { from, commit, json },
        }) => {
            let report = if commit {
                let store = WordbookStore::luma_next_default()?;
                store
                    .import_wordpet(&from, true)
                    .map_err(|e| anyhow::anyhow!("import-wordpet: {e}"))?
            } else {
                WordbookStore::preview_import_wordpet(&from)
                    .map_err(|e| anyhow::anyhow!("import-wordpet: {e}"))?
            };
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "from": from,
                        "committed": report.committed,
                        "would_insert": report.would_insert,
                        "would_update": report.would_update,
                        "skipped": report.skipped,
                        "settings_copied": report.settings_copied,
                        "sample_terms": report.sample_terms,
                    }))?
                );
            } else {
                println!(
                    "import-wordpet {} insert={} update={} skipped={} settings={:?}",
                    if report.committed {
                        "committed"
                    } else {
                        "dry-run"
                    },
                    report.would_insert,
                    report.would_update,
                    report.skipped,
                    report.settings_copied
                );
                if !report.sample_terms.is_empty() {
                    println!("sample: {}", report.sample_terms.join(", "));
                }
            }
        }
        Some(Commands::Wordbook {
            action: WordbookCmd::Backup { json },
        }) => {
            let store = WordbookStore::luma_next_default()?;
            let path = store
                .backup()
                .map_err(|e| anyhow::anyhow!("wordbook backup: {e}"))?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({ "path": path }))?
                );
            } else {
                println!("backed up to {}", path.display());
            }
        }
        Some(Commands::Record { action }) => handle_record_command(action)?,
        Some(Commands::Ssh { action }) => handle_ssh_command(action).await?,
    }
    Ok(())
}

fn records_repo() -> anyhow::Result<SqliteRecordsRepository> {
    let store = Arc::new(RecordsStore::luma_next_default()?);
    Ok(SqliteRecordsRepository::new(store))
}

fn handle_record_command(action: RecordCmd) -> anyhow::Result<()> {
    match action {
        RecordCmd::Status { json } => {
            let repo = records_repo()?;
            let stats = repo.stats().map_err(|e| anyhow::anyhow!("{e}"))?;
            let store = repo.store();
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "db": store.path(),
                        "categories": stats.categories,
                        "records": stats.records,
                    }))?
                );
            } else {
                println!("records.sqlite: {}", store.path().display());
                println!("categories={} records={}", stats.categories, stats.records);
            }
        }
        RecordCmd::Browse { category, json } => {
            let repo = records_repo()?;
            if let Some(cat) = category {
                let rows = repo
                    .list_by_category(&cat, 100)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&rows)?);
                } else {
                    for r in rows {
                        let rating = r
                            .rating
                            .map(|x| x.to_string())
                            .unwrap_or_else(|| "—".into());
                        println!("rec:{}\t{}\t{cat}\t{rating}", r.id, r.name);
                    }
                }
            } else {
                let cats = repo.list_categories().map_err(|e| anyhow::anyhow!("{e}"))?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&cats)?);
                } else {
                    for c in cats {
                        println!("{}\t{}", c.id, c.name);
                    }
                }
            }
        }
        RecordCmd::Add {
            category,
            name,
            rating,
            note,
            json,
        } => {
            let repo = records_repo()?;
            let row = repo
                .insert(&category, &name, rating, note.as_deref().unwrap_or(""))
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&row)?);
            } else {
                println!("added rec:{} {} in {}", row.id, row.name, row.category_name);
            }
        }
        RecordCmd::Rate {
            id,
            score,
            clear,
            json,
        } => {
            if clear && score.is_some() {
                anyhow::bail!("use either score or --clear");
            }
            if !clear && score.is_none() {
                anyhow::bail!("provide SCORE (1-10) or use --clear");
            }
            let rating = if clear { None } else { score };
            let repo = records_repo()?;
            let row = repo
                .set_rating(id, rating)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&row)?);
            } else {
                println!(
                    "rated rec:{} {} → {}",
                    row.id,
                    row.name,
                    row.rating
                        .map(|r| r.to_string())
                        .unwrap_or_else(|| "cleared".into())
                );
            }
        }
        RecordCmd::Note { id, text, json } => {
            let repo = records_repo()?;
            let row = repo
                .set_note(id, &text)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&row)?);
            } else {
                println!("updated note for rec:{} {}", row.id, row.name);
            }
        }
        RecordCmd::Remove { id, yes, json } => {
            if !yes {
                anyhow::bail!("refusing remove without --yes");
            }
            let repo = records_repo()?;
            repo.delete(id).map_err(|e| anyhow::anyhow!("{e}"))?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({ "removed": id }))?
                );
            } else {
                println!("removed rec:{id}");
            }
        }
        RecordCmd::Import { root, apply, json } => {
            if apply {
                let store = RecordsStore::luma_next_default()?;
                let report = import_records_with_ledger(&root, &store, true)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                print_record_import_report(&report, &root, json)?;
            } else {
                let preview = preview_import_from_dir(&root).map_err(|e| anyhow::anyhow!("{e}"))?;
                let migration = record_dry_run(
                    MigrationKind::Records,
                    &root,
                    preview.records as u64,
                    0,
                    preview.errors.len() as u64,
                    preview.warnings.clone(),
                )
                .map_err(|e| anyhow::anyhow!("{e}"))?;
                let report = luma_storage::RecordsImportLedgerReport {
                    preview,
                    apply: None,
                    migration: Some(migration),
                };
                print_record_import_report(&report, &root, json)?;
            }
        }
        RecordCmd::ImportStatus { json } => {
            let repo = records_repo()?;
            let stats = repo.stats().map_err(|e| anyhow::anyhow!("{e}"))?;
            let migrations: Vec<_> = list_migrations()?
                .into_iter()
                .filter(|m| m.kind == luma_storage::MigrationKind::Records)
                .collect();
            let last = migrations.last();
            let config = ConfigStore::luma_next_default()?.load_or_default()?;
            let root = config.records_root.as_deref().unwrap_or("(not set)");
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "categories": stats.categories,
                        "records": stats.records,
                        "records_root": root,
                        "last_migration": last,
                    }))?
                );
            } else {
                println!("categories={} records={}", stats.categories, stats.records);
                println!("records_root={root}");
                if let Some(m) = last {
                    println!(
                        "last import: {} ({:?}) imported={}",
                        m.migration_id, m.status, m.imported
                    );
                } else {
                    println!("last import: (none)");
                }
            }
        }
        RecordCmd::Backup { json } => {
            let store = RecordsStore::luma_next_default()?;
            let path = store.backup().map_err(|e| anyhow::anyhow!("{e}"))?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({ "path": path }))?
                );
            } else {
                println!("backed up to {}", path.display());
            }
        }
    }
    Ok(())
}

fn print_record_import_report(
    report: &luma_storage::RecordsImportLedgerReport,
    root: &std::path::Path,
    json: bool,
) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(report)?);
    } else {
        let p = &report.preview;
        println!("Source: {}", root.display());
        println!(
            "Files found: {} · Categories: {} · Records: {}",
            p.files_found, p.categories, p.records
        );
        println!(
            "Warnings: {} · Errors: {}",
            p.warnings.len(),
            p.errors.len()
        );
        if let Some(apply) = &report.apply {
            println!(
                "Applied: inserted={} skipped={}",
                apply.inserted, apply.skipped
            );
        } else {
            println!("Dry-run (pass --apply to write)");
        }
        if let Some(m) = &report.migration {
            println!("Migration: {}", m.migration_id);
        }
        for w in &p.warnings {
            println!("warning: {w}");
        }
        for e in &p.errors {
            println!("error: {e}");
        }
    }
    Ok(())
}

async fn handle_ssh_command(action: SshCmd) -> anyhow::Result<()> {
    let load = load_registry_with_settings().map_err(|e| anyhow::anyhow!("registry: {e}"))?;
    match action {
        SshCmd::List { json } => {
            let payload = ssh_cli::ssh_list_json(load.registry, Some(load.settings))
                .await
                .map_err(anyhow::Error::msg)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                let results = payload["results"].as_array().cloned().unwrap_or_default();
                for item in results {
                    let id = item["id"].as_str().unwrap_or("");
                    let title = item["title"].as_str().unwrap_or("");
                    let subtitle = item["subtitle"].as_str().unwrap_or("");
                    if subtitle.is_empty() {
                        println!("{id}\t{title}");
                    } else {
                        println!("{id}\t{title}\t{subtitle}");
                    }
                }
            }
        }
        SshCmd::Connect { alias } => {
            let status =
                ssh_cli::ssh_connect_cli(load.registry, &alias, "ssh", Some(load.settings), None)
                    .await
                    .map_err(anyhow::Error::msg)?;
            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        SshCmd::Sftp { alias } => {
            let status =
                ssh_cli::ssh_connect_cli(load.registry, &alias, "sftp", Some(load.settings), None)
                    .await
                    .map_err(anyhow::Error::msg)?;
            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        SshCmd::Favorite { alias } => {
            ssh_cli::ssh_set_favorite(load.registry, Some(load.settings), &alias, true)
                .await
                .map_err(anyhow::Error::msg)?;
        }
        SshCmd::Unfavorite { alias } => {
            ssh_cli::ssh_set_favorite(load.registry, Some(load.settings), &alias, false)
                .await
                .map_err(anyhow::Error::msg)?;
        }
        SshCmd::Rename { alias, name } => {
            ssh_cli::ssh_set_display_name(load.registry, Some(load.settings), &alias, &name)
                .await
                .map_err(anyhow::Error::msg)?;
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
