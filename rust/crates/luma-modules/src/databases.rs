use crate::cancel::await_unless_cancelled;
use async_trait::async_trait;
use luma_application::{
    looks_secret, validate_postgres_metadata, ActionOutcome, ActionRequest, ClockPort,
    DatabasePlatformError, DatabasePlatformPort, DatabasePortal, DatabasePortalTarget,
    DatabasePortalsRepoError, DatabasePortalsRepository, LumaModule, ModuleManifest, ModuleState,
    NewDatabasePortal, OpenPathPort, SearchMode, SearchSink, WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{ActionDescriptorDto, Event, SearchItemDto};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

const MODULE_ID: &str = "luma.databases";
const MAX_COMMAND_CHARS: usize = 4_096;
const MAX_SCHEMA_LABEL_BYTES: usize = 512;
const MAX_SCHEMA_PREVIEW_BYTES: usize = 16 * 1024;
const MAX_LABEL_CHARS: usize = 120;

mod recall;

pub struct DatabasePortalsModule {
    manifest: ModuleManifest,
    repository: Arc<dyn DatabasePortalsRepository>,
    platform: Arc<dyn DatabasePlatformPort>,
    opener: Arc<dyn OpenPathPort>,
    clock: Arc<dyn ClockPort>,
}

impl DatabasePortalsModule {
    /// Canonical command discovery owned by this module, including unavailable fallbacks.
    pub fn command_specs() -> Vec<luma_application::CommandSpec> {
        vec![
            crate::ux::command_spec(
                "/db [query]",
                "List or search configured database portals",
                "/db ",
                None,
            ),
            crate::ux::command_spec(
                "/db add sqlite <label> | <path> [| environment]",
                "Add a canonical local SQLite portal",
                "/db add sqlite ",
                Some("/db add sqlite Local | /tmp/app.sqlite"),
            ),
            crate::ux::command_spec(
                "/db add postgres <label> | <host> | <port> | <database> | <user> [| environment]",
                "Add non-secret PostgreSQL launcher metadata",
                "/db add postgres ",
                Some("/db add postgres Dev | localhost | 5432 | app | me"),
            ),
            crate::ux::command_spec(
                "/db tables <id>",
                "List SQLite tables and indexes",
                "/db tables ",
                Some("/db tables 1"),
            ),
            crate::ux::command_spec(
                "/db schema <id>",
                "Show bounded read-only SQLite DDL",
                "/db schema ",
                Some("/db schema 1"),
            ),
            crate::ux::command_spec(
                "/db remove <id>",
                "Remove portal metadata after confirmation",
                "/db remove ",
                Some("/db remove 1"),
            ),
            crate::ux::command_spec(
                "/db backup",
                "Back up portal metadata only",
                "/db backup",
                None,
            ),
        ]
    }

    pub fn with_deps(
        repository: Arc<dyn DatabasePortalsRepository>,
        platform: Arc<dyn DatabasePlatformPort>,
        opener: Arc<dyn OpenPathPort>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new(MODULE_ID),
                display_name: "Database Portals".into(),
                triggers: vec!["db".into(), "database".into(), "databases".into()],
                default_enabled: false,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
                workbench: luma_application::WorkbenchMeta {
                    glyph: Some("D".into()),
                    suggested_query: Some("/db ".into()),
                    empty_hint: Some("/db add sqlite LABEL | PATH".into()),
                    supports_browse: false,
                    commands: Self::command_specs(),
                },
            },
            repository,
            platform,
            opener,
            clock,
        }
    }

    async fn resolve(
        &self,
        id: i64,
        cancel: &CancellationToken,
    ) -> Result<DatabasePortal, ActionOutcome> {
        if cancel.is_cancelled() {
            return Err(ActionOutcome::Cancelled);
        }
        let portal = self
            .repository
            .get(id)
            .map_err(repo_outcome)?
            .ok_or_else(|| ActionOutcome::Failed {
                kind: FailureKind::NotFound {
                    entity: format!("database portal:{id}"),
                },
            })?;
        if cancel.is_cancelled() {
            return Err(ActionOutcome::Cancelled);
        }
        Ok(portal)
    }

    async fn fresh(
        &self,
        result: &SearchItem,
        cancel: &CancellationToken,
    ) -> Result<DatabasePortal, ActionOutcome> {
        let payload = result
            .action_payload
            .as_ref()
            .ok_or_else(|| invalid_input("missing database portal payload"))?;
        let id = payload
            .get("id")
            .and_then(|value| value.as_i64())
            .ok_or_else(|| invalid_input("missing database portal ID"))?;
        let expected_updated_at = payload
            .get("updated_at")
            .and_then(|value| value.as_str())
            .ok_or_else(|| invalid_input("missing database portal identity"))?;
        let portal = self.resolve(id, cancel).await?;
        if portal.updated_at != expected_updated_at {
            return Err(ActionOutcome::Failed {
                kind: FailureKind::Conflict {
                    reason: "database portal changed; search again before acting".into(),
                },
            });
        }
        Ok(portal)
    }

    async fn revalidate_sqlite(
        &self,
        path: &Path,
        cancel: &CancellationToken,
    ) -> Result<PathBuf, ActionOutcome> {
        let canonical = self
            .platform
            .canonicalize_sqlite(path, cancel.clone())
            .await
            .map_err(platform_outcome)?;
        if canonical != path {
            return Err(ActionOutcome::Failed {
                kind: FailureKind::Conflict {
                    reason: "SQLite target identity changed; search again before acting".into(),
                },
            });
        }
        if cancel.is_cancelled() {
            return Err(ActionOutcome::Cancelled);
        }
        Ok(canonical)
    }

    async fn search_schema(
        &self,
        id: i64,
        include_ddl: bool,
        sink: &SearchSink,
        cancel: &CancellationToken,
    ) {
        let portal = match self.resolve(id, cancel).await {
            Ok(portal) => portal,
            Err(outcome) => {
                send_action_error(sink, outcome).await;
                return;
            }
        };
        let DatabasePortalTarget::Sqlite { path } = &portal.target else {
            send_status(
                sink,
                "db:invalid",
                "PostgreSQL schema browsing is not in this version",
                "Open psql for PostgreSQL schema work",
                "command_error",
            )
            .await;
            return;
        };
        if let Err(outcome) = self.revalidate_sqlite(path, cancel).await {
            send_action_error(sink, outcome).await;
            return;
        }
        let objects = match self.platform.sqlite_schema(path, cancel.clone()).await {
            Ok(objects) => objects,
            Err(DatabasePlatformError::Cancelled) => return,
            Err(error) => {
                send_platform_error(sink, error).await;
                return;
            }
        };
        if cancel.is_cancelled() {
            return;
        }
        let mut upserts = objects
            .into_iter()
            .enumerate()
            .map(|(index, object)| {
                let object_id = stable_schema_hash(&object.kind, &object.name);
                let kind = bounded_display(&object.kind, MAX_SCHEMA_LABEL_BYTES);
                let name = bounded_display(&object.name, MAX_SCHEMA_LABEL_BYTES);
                let table_name = bounded_display(&object.table_name, MAX_SCHEMA_LABEL_BYTES);
                SearchItemDto {
                    id: format!("db:schema:{id}:{object_id:016x}"),
                    module_id: MODULE_ID.into(),
                    title: format!("{kind} · {name}"),
                    subtitle: Some(if include_ddl {
                        if object.ddl.is_empty() {
                            "DDL unavailable".into()
                        } else {
                            bounded_display(&object.ddl, MAX_SCHEMA_PREVIEW_BYTES)
                        }
                    } else if object.kind == "index" {
                        format!("index on {table_name}")
                    } else {
                        "table".into()
                    }),
                    kind: "database_schema".into(),
                    score: 80.0 - index as f64 * 0.01,
                    primary_action_id: "noop".into(),
                    primary_action_label: "OK".into(),
                    ..Default::default()
                }
            })
            .collect::<Vec<_>>();
        if upserts.is_empty() {
            upserts.push(SearchItemDto {
                id: "db:schema-empty".into(),
                module_id: MODULE_ID.into(),
                title: "No user tables or indexes".into(),
                subtitle: Some("The read-only SQLite schema is empty".into()),
                kind: "status".into(),
                score: 0.0,
                primary_action_id: "noop".into(),
                primary_action_label: "OK".into(),
                ..Default::default()
            });
        }
        let _ = sink
            .send(Event::ResultsChunk {
                request_id: String::new(),
                sequence: 1,
                upserts,
                removed_ids: vec![],
            })
            .await;
    }
}

#[async_trait]
impl LumaModule for DatabasePortalsModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, ctx: WarmupContext) -> ModuleState {
        if ctx.cancel.is_cancelled() {
            return ModuleState::Cold;
        }
        match self.repository.list() {
            Ok(_) => ModuleState::Ready,
            Err(error) => ModuleState::Failed(error.to_string()),
        }
    }

    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        if cancel.is_cancelled() {
            return;
        }
        let rest = query.rest_raw().trim();
        if rest.chars().count() > MAX_COMMAND_CHARS {
            send_invalid(&sink, "command exceeds the 4096-character limit").await;
            return;
        }
        if rest.eq_ignore_ascii_case("backup") {
            send_one(
                &sink,
                command_item(
                    "db:backup",
                    "Backup database portal metadata",
                    "No database contents or credentials are copied",
                    "backup",
                    "Backup metadata",
                    ActionRisk::Safe,
                    false,
                    None,
                ),
            )
            .await;
            return;
        }
        if let Some(body) = strip_subcommand(rest, "add") {
            match parse_add(body) {
                Ok(ParsedAdd::Sqlite {
                    label,
                    path,
                    environment,
                }) => {
                    let canonical = match self
                        .platform
                        .canonicalize_sqlite(&path, cancel.clone())
                        .await
                    {
                        Ok(path) => path,
                        Err(DatabasePlatformError::Cancelled) => return,
                        Err(error) => {
                            send_platform_error(&sink, error).await;
                            return;
                        }
                    };
                    if cancel.is_cancelled() {
                        return;
                    }
                    send_one(
                        &sink,
                        command_item(
                            "db:add:sqlite",
                            &format!("Add SQLite portal: {label}"),
                            &format!("{} · {environment}", canonical.display()),
                            "add",
                            "Add portal",
                            ActionRisk::Safe,
                            false,
                            Some(serde_json::json!({
                                "kind": "sqlite",
                                "label": label,
                                "path": canonical,
                                "environment": environment,
                            })),
                        ),
                    )
                    .await;
                }
                Ok(ParsedAdd::Postgres {
                    label,
                    host,
                    port,
                    database,
                    username,
                    environment,
                }) => {
                    send_one(
                        &sink,
                        command_item(
                            "db:add:postgres",
                            &format!("Add PostgreSQL portal: {label}"),
                            &format!(
                                "{username}@{host}:{port}/{database} · {environment} · libpq auth"
                            ),
                            "add",
                            "Add portal",
                            ActionRisk::Safe,
                            false,
                            Some(serde_json::json!({
                                "kind": "postgres",
                                "label": label,
                                "host": host,
                                "port": port,
                                "database": database,
                                "username": username,
                                "environment": environment,
                            })),
                        ),
                    )
                    .await;
                }
                Err(message) => send_invalid(&sink, &message).await,
            }
            return;
        }
        for (command, include_ddl) in [("tables", false), ("schema", true)] {
            if let Some(body) = strip_subcommand(rest, command) {
                match parse_id(body) {
                    Ok(id) => self.search_schema(id, include_ddl, &sink, &cancel).await,
                    Err(message) => send_invalid(&sink, &message).await,
                }
                return;
            }
        }
        if let Some(body) = strip_subcommand(rest, "remove") {
            let id = match parse_id(body) {
                Ok(id) => id,
                Err(message) => {
                    send_invalid(&sink, &message).await;
                    return;
                }
            };
            let portal = match self.resolve(id, &cancel).await {
                Ok(portal) => portal,
                Err(outcome) => {
                    send_action_error(&sink, outcome).await;
                    return;
                }
            };
            send_one(
                &sink,
                command_item(
                    &format!("db:remove:{id}"),
                    &format!("Remove portal metadata: {}", portal.label),
                    "The database file/server/database will not be changed",
                    "remove",
                    "Remove portal",
                    ActionRisk::Destructive,
                    true,
                    Some(identity_payload(&portal)),
                ),
            )
            .await;
            return;
        }

        let portals = match self.repository.list() {
            Ok(portals) => portals,
            Err(error) => {
                send_repo_error(&sink, error).await;
                return;
            }
        };
        if cancel.is_cancelled() {
            return;
        }
        let needle = rest.to_lowercase();
        let mut upserts = portals
            .into_iter()
            .filter(|portal| {
                needle.is_empty()
                    || portal.label.to_lowercase().contains(&needle)
                    || portal.environment.contains(&needle)
            })
            .take(query.limit)
            .enumerate()
            .map(|(index, portal)| portal_item(portal, 80.0 - index as f64 * 0.1))
            .collect::<Vec<_>>();
        if upserts.is_empty() {
            upserts.push(SearchItemDto {
                id: "db:empty".into(),
                module_id: MODULE_ID.into(),
                title: "No database portals".into(),
                subtitle: Some(
                    "/db add sqlite LABEL | PATH · or /db add postgres LABEL | HOST | PORT | DATABASE | USER"
                        .into(),
                ),
                kind: "status".into(),
                score: 0.0,
                primary_action_id: "noop".into(),
                primary_action_label: "OK".into(),
                ..Default::default()
            });
        }
        let _ = sink
            .send(Event::ResultsChunk {
                request_id: String::new(),
                sequence: 1,
                upserts,
                removed_ids: vec![],
            })
            .await;
    }

    async fn actions(&self, result: &SearchItem) -> Vec<ActionDescriptor> {
        if result.kind == "database_portal" {
            let kind = result
                .action_payload
                .as_ref()
                .and_then(|payload| payload.get("kind"))
                .and_then(|value| value.as_str());
            let production = result
                .action_payload
                .as_ref()
                .and_then(|payload| payload.get("environment"))
                .and_then(|value| value.as_str())
                == Some("production");
            let mut actions = vec![if production {
                confirm_action("open_cli", "Open CLI", ActionRisk::Confirm)
            } else {
                safe_action("open_cli", "Open CLI")
            }];
            if kind == Some("sqlite") {
                actions.extend([
                    safe_action("reveal", "Reveal file"),
                    safe_action("tables", "Browse tables"),
                    safe_action("schema", "Preview schema"),
                ]);
            }
            actions.push(confirm_action(
                "remove",
                "Remove portal",
                ActionRisk::Destructive,
            ));
            return actions;
        }
        if result.kind == "database_command" {
            return match result.primary_action.id.as_str() {
                "remove" => vec![confirm_action(
                    "remove",
                    "Remove portal",
                    ActionRisk::Destructive,
                )],
                id @ ("add" | "backup") => vec![safe_action(id, &result.primary_action.label)],
                _ => vec![safe_action("noop", "OK")],
            };
        }
        vec![safe_action("noop", "OK")]
    }

    async fn rehydrate_recall(&self, object_id: &str) -> Result<Option<SearchItem>, String> {
        self.rehydrate_recall_item(object_id).await
    }

    async fn perform(&self, request: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        match request.action.id.as_str() {
            "noop" => ActionOutcome::Success { message: None },
            "add" => {
                let new = match new_portal_from_payload(
                    request.result.action_payload.as_ref(),
                    self.clock.now_rfc3339(),
                ) {
                    Ok(new) => new,
                    Err(outcome) => return outcome,
                };
                let new = match new.target {
                    DatabasePortalTarget::Sqlite { path } => {
                        let canonical = match self
                            .platform
                            .canonicalize_sqlite(&path, cancel.clone())
                            .await
                        {
                            Ok(canonical) => canonical,
                            Err(error) => return platform_outcome(error),
                        };
                        if canonical != path {
                            return ActionOutcome::Failed {
                                kind: FailureKind::Conflict {
                                    reason:
                                        "SQLite target changed after search; run the add command again"
                                            .into(),
                                },
                            };
                        }
                        NewDatabasePortal {
                            target: DatabasePortalTarget::Sqlite { path: canonical },
                            ..new
                        }
                    }
                    DatabasePortalTarget::Postgres {
                        host,
                        port,
                        database,
                        username,
                    } => {
                        if let Err(message) =
                            validate_postgres_metadata(&host, port, &database, &username)
                        {
                            return invalid_input(&message);
                        }
                        NewDatabasePortal {
                            target: DatabasePortalTarget::Postgres {
                                host,
                                port,
                                database,
                                username,
                            },
                            ..new
                        }
                    }
                };
                if cancel.is_cancelled() {
                    return ActionOutcome::Cancelled;
                }
                match self.repository.insert(&new) {
                    Ok(portal) => ActionOutcome::Success {
                        message: Some(format!("added database portal {}", portal.label)),
                    },
                    Err(error) => repo_outcome(error),
                }
            }
            "open_cli" => {
                let portal = match self.fresh(&request.result, &cancel).await {
                    Ok(portal) => portal,
                    Err(outcome) => return outcome,
                };
                if portal.environment == "production" && !request.confirmation {
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "confirmation required before opening a production database"
                                .into(),
                        },
                    };
                }
                if let DatabasePortalTarget::Sqlite { path } = &portal.target {
                    if let Err(outcome) = self.revalidate_sqlite(path, &cancel).await {
                        return outcome;
                    }
                }
                let plan = match self
                    .platform
                    .client_plan(&portal.target, cancel.clone())
                    .await
                {
                    Ok(plan) => plan,
                    Err(error) => return platform_outcome(error),
                };
                if cancel.is_cancelled() {
                    return ActionOutcome::Cancelled;
                }
                ActionOutcome::InteractiveTerminal {
                    program: plan.program,
                    args: plan.args,
                    environment: Vec::new(),
                    record_alias: None,
                }
            }
            "reveal" => {
                let portal = match self.fresh(&request.result, &cancel).await {
                    Ok(portal) => portal,
                    Err(outcome) => return outcome,
                };
                let DatabasePortalTarget::Sqlite { path } = &portal.target else {
                    return invalid_input("only SQLite portals have files to reveal");
                };
                let canonical = match self.revalidate_sqlite(path, &cancel).await {
                    Ok(path) => path,
                    Err(outcome) => return outcome,
                };
                match await_unless_cancelled(&cancel, self.opener.reveal(&canonical)).await {
                    None => ActionOutcome::Cancelled,
                    Some(Ok(())) => ActionOutcome::Success {
                        message: Some("revealed SQLite database".into()),
                    },
                    Some(Err(error)) => unavailable(error.to_string()),
                }
            }
            "tables" | "schema" => {
                let portal = match self.fresh(&request.result, &cancel).await {
                    Ok(portal) => portal,
                    Err(outcome) => return outcome,
                };
                if !matches!(portal.target, DatabasePortalTarget::Sqlite { .. }) {
                    return invalid_input("schema browsing is available for SQLite portals only");
                }
                ActionOutcome::OpenSurface {
                    query: format!("/db {} {}", request.action.id.as_str(), portal.id),
                }
            }
            "remove" => {
                if !request.confirmation {
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "confirmation required to remove portal metadata".into(),
                        },
                    };
                }
                let portal = match self.fresh(&request.result, &cancel).await {
                    Ok(portal) => portal,
                    Err(outcome) => return outcome,
                };
                if cancel.is_cancelled() {
                    return ActionOutcome::Cancelled;
                }
                match self.repository.delete(portal.id, &portal.updated_at) {
                    Ok(()) => ActionOutcome::Success {
                        message: Some(format!(
                            "removed portal metadata for {}; database untouched",
                            portal.label
                        )),
                    },
                    Err(error) => repo_outcome(error),
                }
            }
            "backup" => {
                if cancel.is_cancelled() {
                    return ActionOutcome::Cancelled;
                }
                match self.repository.backup() {
                    Ok(path) => ActionOutcome::Success {
                        message: Some(format!("metadata backup saved to {}", path.display())),
                    },
                    Err(error) => repo_outcome(error),
                }
            }
            _ => invalid_input("unsupported Database Portals action"),
        }
    }

    async fn teardown(&self) {}
}

enum ParsedAdd {
    Sqlite {
        label: String,
        path: PathBuf,
        environment: String,
    },
    Postgres {
        label: String,
        host: String,
        port: u16,
        database: String,
        username: String,
        environment: String,
    },
}

fn parse_add(body: &str) -> Result<ParsedAdd, String> {
    let (kind, fields) = body
        .split_once(char::is_whitespace)
        .ok_or_else(|| "expected `sqlite` or `postgres` followed by fields".to_string())?;
    let fields = fields.split('|').map(str::trim).collect::<Vec<_>>();
    if kind.eq_ignore_ascii_case("sqlite") {
        if !(2..=3).contains(&fields.len()) {
            return Err("usage: /db add sqlite LABEL | PATH [| ENVIRONMENT]".into());
        }
        let label = validate_label(fields[0])?;
        if fields[1].is_empty() || fields[1].chars().any(char::is_control) {
            return Err("SQLite path cannot be empty or contain control characters".into());
        }
        let environment = parse_environment(fields.get(2).copied(), "local")?;
        return Ok(ParsedAdd::Sqlite {
            label,
            path: PathBuf::from(fields[1]),
            environment,
        });
    }
    if kind.eq_ignore_ascii_case("postgres") {
        if !(5..=6).contains(&fields.len()) {
            return Err(
                "usage: /db add postgres LABEL | HOST | PORT | DATABASE | USER [| ENVIRONMENT]"
                    .into(),
            );
        }
        let label = validate_label(fields[0])?;
        let port = fields[2]
            .parse::<u16>()
            .ok()
            .filter(|port| *port > 0)
            .ok_or_else(|| "PostgreSQL port must be between 1 and 65535".to_string())?;
        validate_postgres_metadata(fields[1], port, fields[3], fields[4])?;
        let environment = parse_environment(fields.get(5).copied(), "development")?;
        return Ok(ParsedAdd::Postgres {
            label,
            host: fields[1].into(),
            port,
            database: fields[3].into(),
            username: fields[4].into(),
            environment,
        });
    }
    Err("database kind must be sqlite or postgres".into())
}

fn validate_label(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty()
        || value.chars().count() > MAX_LABEL_CHARS
        || value.chars().any(char::is_control)
    {
        return Err("label must be 1..120 characters without control characters".into());
    }
    if looks_secret(value) || value.contains("://") || value.contains('@') {
        return Err("label looks credential-bearing; use a non-sensitive display label".into());
    }
    Ok(value.into())
}

fn parse_environment(value: Option<&str>, default: &str) -> Result<String, String> {
    let value = value.filter(|value| !value.is_empty()).unwrap_or(default);
    match value.to_ascii_lowercase().as_str() {
        "local" | "development" | "staging" | "production" => Ok(value.to_ascii_lowercase()),
        _ => Err("environment must be local, development, staging, or production".into()),
    }
}

fn new_portal_from_payload(
    payload: Option<&serde_json::Value>,
    now: Result<String, luma_application::ClockError>,
) -> Result<NewDatabasePortal, ActionOutcome> {
    let payload = payload.ok_or_else(|| invalid_input("missing add payload"))?;
    let string = |key: &str| {
        payload
            .get(key)
            .and_then(|value| value.as_str())
            .map(str::to_string)
            .ok_or_else(|| invalid_input(&format!("missing {key}")))
    };
    let label = validate_label(&string("label")?).map_err(|message| invalid_input(&message))?;
    let environment = parse_environment(Some(&string("environment")?), "local")
        .map_err(|message| invalid_input(&message))?;
    let target = match string("kind")?.as_str() {
        "sqlite" => DatabasePortalTarget::Sqlite {
            path: PathBuf::from(string("path")?),
        },
        "postgres" => {
            let host = string("host")?;
            let database = string("database")?;
            let username = string("username")?;
            let port = payload
                .get("port")
                .and_then(|value| value.as_u64())
                .and_then(|port| u16::try_from(port).ok())
                .filter(|port| *port > 0)
                .ok_or_else(|| invalid_input("invalid PostgreSQL port"))?;
            validate_postgres_metadata(&host, port, &database, &username)
                .map_err(|message| invalid_input(&message))?;
            DatabasePortalTarget::Postgres {
                host,
                port,
                database,
                username,
            }
        }
        _ => return Err(invalid_input("unsupported database kind")),
    };
    Ok(NewDatabasePortal {
        label,
        target,
        environment,
        now: now.map_err(|error| unavailable(error.to_string()))?,
    })
}

fn strip_subcommand<'a>(rest: &'a str, command: &str) -> Option<&'a str> {
    let (head, tail) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
    head.eq_ignore_ascii_case(command).then_some(tail.trim())
}

fn parse_id(value: &str) -> Result<i64, String> {
    if value.split_whitespace().count() != 1 {
        return Err("command requires exactly one portal ID".into());
    }
    value
        .parse::<i64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| "portal ID must be a positive integer".into())
}

fn bounded_display(value: &str, max_bytes: usize) -> String {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.len() <= max_bytes {
        return compact;
    }
    let mut end = max_bytes;
    while end > 0 && !compact.is_char_boundary(end) {
        end -= 1;
    }
    compact[..end].into()
}

fn stable_schema_hash(kind: &str, name: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in kind
        .as_bytes()
        .iter()
        .copied()
        .chain([0])
        .chain(name.as_bytes().iter().copied())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn portal_item(portal: DatabasePortal, score: f64) -> SearchItemDto {
    let (kind, target) = match &portal.target {
        DatabasePortalTarget::Sqlite { path } => ("sqlite", path.display().to_string()),
        DatabasePortalTarget::Postgres {
            host,
            port,
            database,
            username,
        } => (
            "postgres",
            format!("{username}@{host}:{port}/{database} · libpq auth"),
        ),
    };
    let production = portal.environment == "production";
    SearchItemDto {
        id: format!("db:{}", portal.id),
        module_id: MODULE_ID.into(),
        title: portal.label.clone(),
        subtitle: Some(format!("{kind} · {} · {target}", portal.environment)),
        kind: "database_portal".into(),
        score,
        primary_action_id: "open_cli".into(),
        primary_action_label: "Open CLI".into(),
        primary_action_risk: if production {
            ActionRisk::Confirm
        } else {
            ActionRisk::Safe
        },
        primary_action_confirmation: production,
        secondary_actions: if kind == "sqlite" {
            vec![
                action_dto("reveal", "Reveal file", ActionRisk::Safe, false),
                action_dto("tables", "Browse tables", ActionRisk::Safe, false),
                action_dto("schema", "Preview schema", ActionRisk::Safe, false),
                action_dto("remove", "Remove portal", ActionRisk::Destructive, true),
            ]
        } else {
            vec![action_dto(
                "remove",
                "Remove portal",
                ActionRisk::Destructive,
                true,
            )]
        },
        action_payload: Some(serde_json::json!({
            "id": portal.id,
            "updated_at": portal.updated_at,
            "kind": kind,
            "environment": portal.environment,
        })),
        ..Default::default()
    }
}

fn identity_payload(portal: &DatabasePortal) -> serde_json::Value {
    serde_json::json!({
        "id": portal.id,
        "updated_at": portal.updated_at,
        "kind": match &portal.target {
            DatabasePortalTarget::Sqlite { .. } => "sqlite",
            DatabasePortalTarget::Postgres { .. } => "postgres",
        },
        "environment": portal.environment,
    })
}

#[allow(clippy::too_many_arguments)]
fn command_item(
    id: &str,
    title: &str,
    subtitle: &str,
    action: &str,
    action_label: &str,
    risk: ActionRisk,
    confirmation: bool,
    payload: Option<serde_json::Value>,
) -> SearchItemDto {
    SearchItemDto {
        id: id.into(),
        module_id: MODULE_ID.into(),
        title: title.into(),
        subtitle: Some(subtitle.into()),
        kind: "database_command".into(),
        score: 100.0,
        primary_action_id: action.into(),
        primary_action_label: action_label.into(),
        primary_action_risk: risk,
        primary_action_confirmation: confirmation,
        action_payload: payload,
        ..Default::default()
    }
}

fn action_dto(id: &str, label: &str, risk: ActionRisk, confirmation: bool) -> ActionDescriptorDto {
    ActionDescriptorDto {
        id: id.into(),
        label: label.into(),
        risk,
        confirmation,
    }
}

fn safe_action(id: &str, label: &str) -> ActionDescriptor {
    ActionDescriptor {
        id: ActionId::new(id),
        label: label.into(),
        risk: ActionRisk::Safe,
        confirmation: false,
    }
}

fn confirm_action(id: &str, label: &str, risk: ActionRisk) -> ActionDescriptor {
    ActionDescriptor {
        id: ActionId::new(id),
        label: label.into(),
        risk,
        confirmation: true,
    }
}

fn invalid_input(message: &str) -> ActionOutcome {
    ActionOutcome::Failed {
        kind: FailureKind::InvalidInput {
            field: "database portal".into(),
            message: message.into(),
        },
    }
}

fn unavailable(reason: String) -> ActionOutcome {
    ActionOutcome::Failed {
        kind: FailureKind::Unavailable {
            reason,
            retryable: true,
        },
    }
}

fn repo_outcome(error: DatabasePortalsRepoError) -> ActionOutcome {
    ActionOutcome::Failed {
        kind: match error {
            DatabasePortalsRepoError::NotFound => FailureKind::NotFound {
                entity: "database portal".into(),
            },
            DatabasePortalsRepoError::Conflict => FailureKind::Conflict {
                reason: "database portal changed; search again before acting".into(),
            },
            DatabasePortalsRepoError::Duplicate => FailureKind::Conflict {
                reason: "a database portal already uses that label".into(),
            },
            DatabasePortalsRepoError::Capacity => FailureKind::Conflict {
                reason: format!(
                    "database portals capacity reached ({})",
                    luma_application::MAX_DATABASE_PORTALS
                ),
            },
            DatabasePortalsRepoError::Store(reason) => FailureKind::Unavailable {
                reason,
                retryable: true,
            },
        },
    }
}

fn platform_outcome(error: DatabasePlatformError) -> ActionOutcome {
    if error == DatabasePlatformError::Cancelled {
        return ActionOutcome::Cancelled;
    }
    ActionOutcome::Failed {
        kind: match error {
            DatabasePlatformError::Cancelled => unreachable!(),
            DatabasePlatformError::NotConfigured(remediation) => {
                FailureKind::NotConfigured { remediation }
            }
            DatabasePlatformError::NotFound => FailureKind::NotFound {
                entity: "database target".into(),
            },
            DatabasePlatformError::Invalid(message) => FailureKind::InvalidInput {
                field: "database target".into(),
                message,
            },
            DatabasePlatformError::Unavailable(reason) => FailureKind::Unavailable {
                reason,
                retryable: true,
            },
        },
    }
}

async fn send_one(sink: &SearchSink, item: SearchItemDto) {
    let _ = sink
        .send(Event::ResultsChunk {
            request_id: String::new(),
            sequence: 1,
            upserts: vec![item],
            removed_ids: vec![],
        })
        .await;
}

async fn send_status(sink: &SearchSink, id: &str, title: &str, subtitle: &str, kind: &str) {
    send_one(
        sink,
        SearchItemDto {
            id: id.into(),
            module_id: MODULE_ID.into(),
            title: title.into(),
            subtitle: Some(subtitle.into()),
            kind: kind.into(),
            score: 0.0,
            primary_action_id: "noop".into(),
            primary_action_label: "OK".into(),
            ..Default::default()
        },
    )
    .await;
}

async fn send_invalid(sink: &SearchSink, message: &str) {
    send_status(
        sink,
        "db:invalid",
        "Database Portals command is invalid",
        message,
        "command_error",
    )
    .await;
}

async fn send_repo_error(sink: &SearchSink, error: DatabasePortalsRepoError) {
    send_status(
        sink,
        "db:unavailable",
        "Database portal metadata is unavailable",
        &error.to_string(),
        "unavailable",
    )
    .await;
}

async fn send_platform_error(sink: &SearchSink, error: DatabasePlatformError) {
    let (title, kind) = match error {
        DatabasePlatformError::NotConfigured(_) => {
            ("Database client is not configured", "not_configured")
        }
        DatabasePlatformError::NotFound => ("Database target was not found", "not_configured"),
        DatabasePlatformError::Invalid(_) => ("Database target is invalid", "command_error"),
        DatabasePlatformError::Unavailable(_) => {
            ("Database operation is unavailable", "unavailable")
        }
        DatabasePlatformError::Cancelled => return,
    };
    send_status(sink, "db:platform-error", title, &error.to_string(), kind).await;
}

async fn send_action_error(sink: &SearchSink, outcome: ActionOutcome) {
    match outcome {
        ActionOutcome::Cancelled => {}
        ActionOutcome::Failed { kind } => {
            send_status(
                sink,
                "db:failed",
                "Database portal operation failed",
                &kind.user_message(),
                "command_error",
            )
            .await
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_application::{
        FakeDatabasePlatform, FakeOpenPath, FixedClock, MemoryDatabasePortalsRepository,
    };
    use luma_domain::ResultId;

    fn clock() -> Arc<FixedClock> {
        Arc::new(FixedClock::new("2026-01-01", "v1"))
    }

    fn insert_portal(
        repository: &MemoryDatabasePortalsRepository,
        target: DatabasePortalTarget,
        environment: &str,
    ) -> DatabasePortal {
        repository
            .insert(&NewDatabasePortal {
                label: "Portal".into(),
                target,
                environment: environment.into(),
                now: "v1".into(),
            })
            .unwrap()
    }

    fn item(portal: &DatabasePortal, action: &str, risk: ActionRisk) -> SearchItem {
        let confirmation = risk != ActionRisk::Safe;
        SearchItem {
            id: ResultId::new(format!("db:{}", portal.id)),
            module_id: ModuleId::new(MODULE_ID),
            title: portal.label.clone(),
            subtitle: None,
            kind: "database_portal".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new(action),
                label: action.into(),
                risk,
                confirmation,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: Some(identity_payload(portal)),
        }
    }

    #[tokio::test]
    async fn recall_rehydration_restores_current_database_identity_and_risk() {
        let repository = Arc::new(MemoryDatabasePortalsRepository::default());
        let path = PathBuf::from("/fixture/production.sqlite");
        let portal = insert_portal(
            &repository,
            DatabasePortalTarget::Sqlite { path: path.clone() },
            "production",
        );
        let module = DatabasePortalsModule::with_deps(
            repository.clone(),
            Arc::new(FakeDatabasePlatform::new(path)),
            Arc::new(FakeOpenPath::new()),
            clock(),
        );

        let item = module
            .rehydrate_recall(&format!("db:{}", portal.id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(item.primary_action.risk, ActionRisk::Confirm);
        assert!(item.primary_action.confirmation);
        assert_eq!(
            item.action_payload.as_ref().unwrap()["updated_at"],
            portal.updated_at
        );

        repository.delete(portal.id, &portal.updated_at).unwrap();
        assert!(module
            .rehydrate_recall(&format!("db:{}", portal.id))
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn sqlite_open_revalidates_path_and_returns_exact_plan() {
        let repository = Arc::new(MemoryDatabasePortalsRepository::default());
        let path = PathBuf::from("/fixture/app.sqlite");
        let portal = insert_portal(
            &repository,
            DatabasePortalTarget::Sqlite { path: path.clone() },
            "local",
        );
        let module = DatabasePortalsModule::with_deps(
            repository,
            Arc::new(FakeDatabasePlatform::new(path)),
            Arc::new(FakeOpenPath::new()),
            clock(),
        );
        let outcome = module
            .perform(
                ActionRequest {
                    result: item(&portal, "open_cli", ActionRisk::Safe),
                    action: safe_action("open_cli", "Open CLI"),
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert_eq!(
            outcome,
            ActionOutcome::InteractiveTerminal {
                program: "/fixture/sqlite3".into(),
                args: vec!["/fixture/app.sqlite".into()],
                environment: Vec::new(),
                record_alias: None,
            }
        );
    }

    #[tokio::test]
    async fn postgres_plan_is_direct_and_payload_has_no_password_or_dsn() {
        let repository = Arc::new(MemoryDatabasePortalsRepository::default());
        let portal = insert_portal(
            &repository,
            DatabasePortalTarget::Postgres {
                host: "db.example.test".into(),
                port: 5432,
                database: "app".into(),
                username: "reader".into(),
            },
            "staging",
        );
        let module = DatabasePortalsModule::with_deps(
            repository,
            Arc::new(FakeDatabasePlatform::new(PathBuf::from("/fixture/unused"))),
            Arc::new(FakeOpenPath::new()),
            clock(),
        );
        let outcome = module
            .perform(
                ActionRequest {
                    result: item(&portal, "open_cli", ActionRisk::Safe),
                    action: safe_action("open_cli", "Open CLI"),
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        let ActionOutcome::InteractiveTerminal { args, .. } = outcome else {
            panic!("expected terminal plan");
        };
        assert_eq!(
            args,
            vec![
                "--host",
                "db.example.test",
                "--port",
                "5432",
                "--username",
                "reader",
                "--dbname",
                "app"
            ]
        );
        let payload = serde_json::to_string(&identity_payload(&portal)).unwrap();
        assert!(!payload.to_lowercase().contains("password"));
        assert!(!payload.contains("://"));
    }

    #[tokio::test]
    async fn production_open_requires_confirmation_before_platform_call() {
        let repository = Arc::new(MemoryDatabasePortalsRepository::default());
        let portal = insert_portal(
            &repository,
            DatabasePortalTarget::Postgres {
                host: "prod.example.test".into(),
                port: 5432,
                database: "app".into(),
                username: "reader".into(),
            },
            "production",
        );
        let platform = Arc::new(FakeDatabasePlatform::new(PathBuf::from("/fixture/unused")));
        let module = DatabasePortalsModule::with_deps(
            repository,
            platform.clone(),
            Arc::new(FakeOpenPath::new()),
            clock(),
        );
        let outcome = module
            .perform(
                ActionRequest {
                    result: item(&portal, "open_cli", ActionRisk::Confirm),
                    action: confirm_action("open_cli", "Open", ActionRisk::Confirm),
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(
            outcome,
            ActionOutcome::Failed {
                kind: FailureKind::SecurityDenied { .. }
            }
        ));
        assert!(platform.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn stale_and_cancelled_actions_have_no_platform_side_effect() {
        let repository = Arc::new(MemoryDatabasePortalsRepository::default());
        let portal = insert_portal(
            &repository,
            DatabasePortalTarget::Sqlite {
                path: PathBuf::from("/fixture/app.sqlite"),
            },
            "local",
        );
        let mut stale_portal = portal.clone();
        stale_portal.updated_at = "stale".into();
        let platform = Arc::new(FakeDatabasePlatform::new(PathBuf::from(
            "/fixture/app.sqlite",
        )));
        let module = DatabasePortalsModule::with_deps(
            repository,
            platform.clone(),
            Arc::new(FakeOpenPath::new()),
            clock(),
        );
        let stale = module
            .perform(
                ActionRequest {
                    result: item(&stale_portal, "reveal", ActionRisk::Safe),
                    action: safe_action("reveal", "Reveal"),
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(
            stale,
            ActionOutcome::Failed {
                kind: FailureKind::Conflict { .. }
            }
        ));
        assert!(platform.calls.lock().unwrap().is_empty());
        let cancel = CancellationToken::new();
        cancel.cancel();
        assert_eq!(
            module
                .perform(
                    ActionRequest {
                        result: item(&portal, "open_cli", ActionRisk::Safe),
                        action: safe_action("open_cli", "Open"),
                        confirmation: false,
                    },
                    cancel,
                )
                .await,
            ActionOutcome::Cancelled
        );
    }

    #[test]
    fn schema_identity_is_hashed_and_display_text_is_bounded() {
        let raw = format!("name\n{}", "界".repeat(1_000));
        let display = bounded_display(&raw, MAX_SCHEMA_LABEL_BYTES);
        assert!(display.len() <= MAX_SCHEMA_LABEL_BYTES);
        assert!(!display.contains('\n'));
        assert_eq!(
            stable_schema_hash("table", "users"),
            stable_schema_hash("table", "users")
        );
        assert_ne!(
            stable_schema_hash("table", "users"),
            stable_schema_hash("index", "users")
        );
    }

    #[test]
    fn parser_rejects_password_dsns_and_secret_labels() {
        assert!(parse_add("postgres Prod | postgres://u:p@host/db | 5432 | app | reader").is_err());
        assert!(parse_add("postgres Password=x | host | 5432 | app | reader").is_err());
        assert!(parse_add("postgres Prod | host | 5432 | password=x | reader").is_err());
    }
}
