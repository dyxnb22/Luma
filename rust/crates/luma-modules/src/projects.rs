use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, ImportedProject, LumaModule, ModuleManifest, ModuleState,
    OpenPathPort, ProjectDirectoryListing, ProjectOpenScope, ProjectWorkspaceError,
    ProjectWorkspacePort, SearchMode, SearchSink, WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{Event, SearchItemDto, UiIntent};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use crate::cancel::await_unless_cancelled;

#[derive(Clone)]
struct Project {
    name: String,
    path: PathBuf,
    missing: bool,
}

pub struct ProjectsModule {
    manifest: ModuleManifest,
    roots: Arc<RwLock<Vec<PathBuf>>>,
    imported: Arc<RwLock<Vec<ImportedProject>>>,
    opener: Arc<dyn OpenPathPort>,
    workspace: Arc<dyn ProjectWorkspacePort>,
}

impl ProjectsModule {
    pub fn with_roots(
        roots: Vec<PathBuf>,
        opener: Arc<dyn OpenPathPort>,
        workspace: Arc<dyn ProjectWorkspacePort>,
    ) -> Self {
        Self::with_settings(roots, Vec::new(), opener, workspace)
    }

    pub fn with_settings(
        roots: Vec<PathBuf>,
        imported: Vec<ImportedProject>,
        opener: Arc<dyn OpenPathPort>,
        workspace: Arc<dyn ProjectWorkspacePort>,
    ) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.projects"),
                display_name: "Projects".into(),
                triggers: vec!["p".into(), "proj".into(), "project".into()],
                default_enabled: true,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
                workbench: luma_application::WorkbenchMeta {
                    glyph: Some("P".into()),
                    suggested_query: Some("proj browse".into()),
                    empty_hint: Some(
                        "proj browse · proj add PATH · proj <name> · Hub Enter opens browse".into(),
                    ),
                    supports_browse: true,
                },
            },
            roots: Arc::new(RwLock::new(roots)),
            imported: Arc::new(RwLock::new(imported)),
            opener,
            workspace,
        }
    }

    async fn is_imported_path(&self, path: &Path) -> bool {
        let canonical_path = path.display().to_string();
        self.imported
            .read()
            .await
            .iter()
            .any(|project| project.path == canonical_path)
    }
}

fn imported_index(imported: &[ImportedProject], statuses: &[bool]) -> Vec<Project> {
    imported
        .iter()
        .enumerate()
        .map(|(index, p)| {
            let path = PathBuf::from(&p.path);
            let missing = !statuses.get(index).copied().unwrap_or(false);
            let name = p.name.clone().unwrap_or_else(|| {
                path.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("project")
                    .to_string()
            });
            Project {
                name,
                path,
                missing,
            }
        })
        .collect()
}

fn format_projects_directory_preview(listing: ProjectDirectoryListing) -> String {
    const MAX: usize = 40;
    if listing.entries.is_empty() && !listing.truncated {
        return "Empty folder".into();
    }
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    for entry in listing.entries {
        if entry.is_directory {
            dirs.push(entry.name);
        } else {
            files.push(entry.name);
        }
    }
    let total = dirs.len() + files.len();
    let mut out = if listing.truncated {
        format!("At least {total} item(s):\n")
    } else {
        format!("{total} item(s):\n")
    };
    let mut shown = 0usize;
    for d in &dirs {
        if shown >= MAX {
            break;
        }
        out.push_str(&format!("  {d}/\n"));
        shown += 1;
    }
    for f in &files {
        if shown >= MAX {
            break;
        }
        out.push_str(&format!("  {f}\n"));
        shown += 1;
    }
    if shown < total {
        out.push_str(&format!("  … +{} more\n", total - shown));
    } else if listing.truncated {
        out.push_str("  … more not shown\n");
    }
    out.trim_end().to_string()
}

#[async_trait]
impl LumaModule for ProjectsModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
        ModuleState::Ready
    }

    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        let roots = self.roots.read().await.clone();
        let imported = self.imported.read().await.clone();
        let rest_norm = query
            .normalized
            .split_once(|c: char| c.is_whitespace())
            .map(|(_, r)| r.trim().to_string())
            .unwrap_or_default();
        let rest_raw = query.rest_raw().trim().to_string();
        let rest_check = rest_raw.to_lowercase();

        // Drill-down: verb case-insensitive; path payload from rest_raw (preserve case).
        if rest_check == "browse"
            || rest_check.starts_with("browse ")
            || rest_check.starts_with("ls ")
        {
            if roots.is_empty() {
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![SearchItemDto {
                            id: "proj:configure".into(),
                            module_id: "luma.projects".into(),
                            title: "Add a project browse root".into(),
                            subtitle: Some("Run: luma config set --projects-root ~/dev".into()),
                            kind: "not_configured".into(),
                            score: 0.0,
                            primary_action_id: "seed_config".into(),
                            primary_action_label: "Show command".into(),
                            ui_intent: Some(UiIntent::SeedConfig),
                            action_payload: None,
                            ..Default::default()
                        }],
                        removed_ids: vec![],
                    })
                    .await;
                return;
            }
            let path_arg = rest_raw
                .strip_prefix("browse")
                .or_else(|| rest_raw.strip_prefix("Browse"))
                .or_else(|| rest_raw.strip_prefix("BROWSE"))
                .or_else(|| rest_raw.strip_prefix("ls"))
                .or_else(|| rest_raw.strip_prefix("LS"))
                .unwrap_or("")
                .trim();
            let target = if path_arg.is_empty() {
                None
            } else {
                Some(PathBuf::from(path_arg))
            };
            let browse_label = target
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "project roots".into());
            let mut upserts = Vec::new();
            let mut listing_truncated = false;
            if let Some(dir) = target {
                let denied_label = dir.display().to_string();
                let dir = match await_unless_cancelled(
                    &cancel,
                    self.workspace
                        .resolve_browse_path(dir, roots.clone(), cancel.clone()),
                )
                .await
                {
                    None | Some(Err(ProjectWorkspaceError::Cancelled)) => return,
                    Some(Ok(dir)) => dir,
                    Some(Err(_)) => {
                        let _ = sink
                            .send(Event::ResultsChunk {
                                request_id: String::new(),
                                sequence: 1,
                                upserts: vec![SearchItemDto {
                                    id: "proj:denied".into(),
                                    module_id: "luma.projects".into(),
                                    title: "Path outside project roots".into(),
                                    subtitle: Some(denied_label),
                                    kind: "unavailable".into(),
                                    score: 0.0,
                                    primary_action_id: "noop".into(),
                                    primary_action_label: "Unavailable".into(),
                                    ..Default::default()
                                }],
                                removed_ids: vec![],
                            })
                            .await;
                        return;
                    }
                };
                let listing = match await_unless_cancelled(
                    &cancel,
                    self.workspace.list_directory(dir, cancel.clone()),
                )
                .await
                {
                    None | Some(Err(ProjectWorkspaceError::Cancelled)) => return,
                    Some(Ok(listing)) => listing,
                    // Preserve browse's historical empty-folder result for a missing drill-down
                    // target; it remains safely contained because resolution happened first.
                    Some(Err(ProjectWorkspaceError::NotFound(_))) => {
                        ProjectDirectoryListing::default()
                    }
                    Some(Err(error)) => {
                        let _ = sink
                            .send(Event::ResultsChunk {
                                request_id: String::new(),
                                sequence: 1,
                                upserts: vec![SearchItemDto {
                                    id: "proj:browse-unavailable".into(),
                                    module_id: "luma.projects".into(),
                                    title: "Project folder unavailable".into(),
                                    subtitle: Some(error.to_string()),
                                    kind: "unavailable".into(),
                                    score: 0.0,
                                    primary_action_id: "noop".into(),
                                    primary_action_label: "Unavailable".into(),
                                    ..Default::default()
                                }],
                                removed_ids: vec![],
                            })
                            .await;
                        return;
                    }
                };
                listing_truncated = listing.truncated;
                for entry in listing.entries {
                    if cancel.is_cancelled() {
                        return;
                    }
                    let path = entry.path;
                    if entry.is_directory {
                        let imported = self.is_imported_path(&path).await;
                        upserts.push(SearchItemDto {
                            id: format!("browse:proj:{}", path.display()),
                            module_id: "luma.projects".into(),
                            title: format!("{}/", entry.name),
                            subtitle: Some(path.display().to_string()),
                            kind: if imported {
                                "imported".into()
                            } else {
                                "directory".into()
                            },
                            score: 80.0,
                            primary_action_id: if imported {
                                "browse".into()
                            } else {
                                "import_project".into()
                            },
                            primary_action_label: if imported {
                                "Browse".into()
                            } else {
                                "Import project".into()
                            },
                            ui_intent: Some(UiIntent::Browse).filter(|_| imported),
                            action_payload: None,
                            ..Default::default()
                        });
                    } else {
                        upserts.push(SearchItemDto {
                            id: format!("proj:{}", path.display()),
                            module_id: "luma.projects".into(),
                            title: entry.name,
                            subtitle: Some(path.display().to_string()),
                            kind: "file".into(),
                            score: 70.0,
                            primary_action_id: "open".into(),
                            primary_action_label: "Open".into(),
                            ..Default::default()
                        });
                    }
                }
            } else {
                for root in &roots {
                    upserts.push(SearchItemDto {
                        id: format!("browse:proj:{}", root.display()),
                        module_id: "luma.projects".into(),
                        title: format!(
                            "{}/",
                            root.file_name().and_then(|s| s.to_str()).unwrap_or("root")
                        ),
                        subtitle: Some(root.display().to_string()),
                        kind: "directory".into(),
                        score: 90.0,
                        primary_action_id: "browse".into(),
                        primary_action_label: "Browse".into(),
                        ui_intent: Some(UiIntent::Browse),
                        action_payload: None,
                        ..Default::default()
                    });
                }
            }
            if upserts.is_empty() {
                upserts.push(SearchItemDto {
                    id: if listing_truncated {
                        "proj:browse-truncated".into()
                    } else {
                        "proj:browse-empty".into()
                    },
                    module_id: "luma.projects".into(),
                    title: if listing_truncated {
                        "Folder listing is limited".into()
                    } else {
                        "Empty folder".into()
                    },
                    subtitle: Some(browse_label),
                    kind: "status".into(),
                    score: 50.0,
                    primary_action_id: "noop".into(),
                    primary_action_label: "OK".into(),
                    ..Default::default()
                });
            }
            upserts.truncate(query.limit);
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts,
                    removed_ids: vec![],
                })
                .await;
            return;
        }

        if rest_check == "add"
            || rest_check == "import"
            || rest_check.starts_with("add ")
            || rest_check.starts_with("import ")
        {
            let path_str = rest_raw
                .split_once(char::is_whitespace)
                .map(|(_, tail)| tail.trim())
                .unwrap_or("");
            if path_str.is_empty() {
                let usage = if rest_check == "import" {
                    "Usage: proj import /path/to/project"
                } else {
                    "Usage: proj add /path/to/project"
                };
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![SearchItemDto {
                            id: "proj:import-usage".into(),
                            module_id: "luma.projects".into(),
                            title: "Import a project directory".into(),
                            subtitle: Some(usage.into()),
                            kind: "status".into(),
                            score: 50.0,
                            primary_action_id: "noop".into(),
                            primary_action_label: "OK".into(),
                            ..Default::default()
                        }],
                        removed_ids: vec![],
                    })
                    .await;
                return;
            }
            let path = PathBuf::from(path_str);
            match await_unless_cancelled(
                &cancel,
                self.workspace.resolve_import_path(path, cancel.clone()),
            )
            .await
            {
                None | Some(Err(ProjectWorkspaceError::Cancelled)) => return,
                Some(Ok(canon)) => {
                    let title = canon
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("project")
                        .to_string();
                    let _ = sink
                        .send(Event::ResultsChunk {
                            request_id: String::new(),
                            sequence: 1,
                            upserts: vec![SearchItemDto {
                                id: format!("proj:import:{}", canon.display()),
                                module_id: "luma.projects".into(),
                                title: format!("Import {title}"),
                                subtitle: Some(canon.display().to_string()),
                                kind: "command".into(),
                                score: 95.0,
                                primary_action_id: "import_project".into(),
                                primary_action_label: "Import".into(),
                                action_payload: Some(serde_json::json!({
                                    "path": canon.display().to_string()
                                })),
                                ..Default::default()
                            }],
                            removed_ids: vec![],
                        })
                        .await;
                }
                Some(Err(err)) => {
                    let _ = sink
                        .send(Event::ResultsChunk {
                            request_id: String::new(),
                            sequence: 1,
                            upserts: vec![SearchItemDto {
                                id: "proj:import-denied".into(),
                                module_id: "luma.projects".into(),
                                title: "Cannot import project".into(),
                                subtitle: Some(err.to_string()),
                                kind: "unavailable".into(),
                                score: 0.0,
                                primary_action_id: "noop".into(),
                                primary_action_label: "Unavailable".into(),
                                ..Default::default()
                            }],
                            removed_ids: vec![],
                        })
                        .await;
                }
            }
            return;
        }

        if rest_check == "remove" || rest_check.starts_with("remove ") {
            let key = rest_raw
                .split_once(char::is_whitespace)
                .map(|(_, tail)| tail.trim())
                .unwrap_or("");
            if key.is_empty() {
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![SearchItemDto {
                            id: "proj:remove-usage".into(),
                            module_id: "luma.projects".into(),
                            title: "Remove an imported project".into(),
                            subtitle: Some("Usage: proj remove project-name".into()),
                            kind: "status".into(),
                            score: 50.0,
                            primary_action_id: "noop".into(),
                            primary_action_label: "OK".into(),
                            ..Default::default()
                        }],
                        removed_ids: vec![],
                    })
                    .await;
                return;
            }
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![SearchItemDto {
                        id: format!("proj:remove:{key}"),
                        module_id: "luma.projects".into(),
                        title: format!("Remove {key}"),
                        subtitle: Some("Removes import config only — not the directory".into()),
                        kind: "command".into(),
                        score: 95.0,
                        primary_action_id: "remove_project".into(),
                        primary_action_label: "Remove".into(),
                        primary_action_confirmation: true,
                        primary_action_risk: ActionRisk::Confirm,
                        action_payload: Some(serde_json::json!({ "name": key })),
                        ..Default::default()
                    }],
                    removed_ids: vec![],
                })
                .await;
            return;
        }

        let needle = rest_norm;
        if imported.is_empty() && needle.is_empty() {
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![SearchItemDto {
                        id: "proj:not-configured".into(),
                        module_id: "luma.projects".into(),
                        title: "No imported projects".into(),
                        subtitle: Some(
                            "proj add /path · proj browse · luma config set --projects-root ~/dev"
                                .into(),
                        ),
                        kind: "not_configured".into(),
                        score: 0.0,
                        primary_action_id: "seed_config".into(),
                        primary_action_label: "Show command".into(),
                        ui_intent: Some(UiIntent::SeedConfig),
                        action_payload: None,
                        ..Default::default()
                    }],
                    removed_ids: vec![],
                })
                .await;
            return;
        }

        let imported_paths = imported
            .iter()
            .map(|project| PathBuf::from(&project.path))
            .collect();
        let statuses = match await_unless_cancelled(
            &cancel,
            self.workspace
                .imported_project_statuses(imported_paths, cancel.clone()),
        )
        .await
        {
            None | Some(Err(ProjectWorkspaceError::Cancelled)) => return,
            Some(Ok(statuses)) => statuses,
            Some(Err(error)) => {
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![SearchItemDto {
                            id: "proj:workspace-unavailable".into(),
                            module_id: "luma.projects".into(),
                            title: "Project workspace unavailable".into(),
                            subtitle: Some(error.to_string()),
                            kind: "unavailable".into(),
                            score: 0.0,
                            primary_action_id: "noop".into(),
                            primary_action_label: "Unavailable".into(),
                            ..Default::default()
                        }],
                        removed_ids: vec![],
                    })
                    .await;
                return;
            }
        };
        let index = imported_index(&imported, &statuses);
        let mut upserts = Vec::new();
        for p in index {
            if cancel.is_cancelled() {
                return;
            }
            if needle.is_empty() || p.name.to_lowercase().contains(&needle) {
                let (kind, primary, label) = if p.missing {
                    ("unavailable", "remove_project", "Remove missing project")
                } else {
                    ("project", "open", "Open")
                };
                upserts.push(SearchItemDto {
                    id: format!("proj:{}", p.path.display()),
                    module_id: "luma.projects".into(),
                    title: p.name.clone(),
                    subtitle: Some(if p.missing {
                        format!("{} — path missing", p.path.display())
                    } else {
                        p.path.display().to_string()
                    }),
                    kind: kind.into(),
                    score: if p.missing { 40.0 } else { 65.0 },
                    primary_action_id: primary.into(),
                    primary_action_label: label.into(),
                    primary_action_confirmation: p.missing,
                    primary_action_risk: if p.missing {
                        ActionRisk::Confirm
                    } else {
                        ActionRisk::Safe
                    },
                    action_payload: if p.missing {
                        Some(serde_json::json!({ "name": p.name }))
                    } else {
                        None
                    },
                    ..Default::default()
                });
            }
        }
        if upserts.is_empty() {
            let (title, subtitle) = if needle.is_empty() {
                (
                    "No imported projects".into(),
                    "proj add /path · proj browse".into(),
                )
            } else {
                (
                    format!("No projects matching \"{needle}\""),
                    "Try another query · proj browse".into(),
                )
            };
            upserts.push(SearchItemDto {
                id: "proj:no-matches".into(),
                module_id: "luma.projects".into(),
                title,
                subtitle: Some(subtitle),
                kind: "status".into(),
                score: 50.0,
                primary_action_id: "noop".into(),
                primary_action_label: "OK".into(),
                ..Default::default()
            });
        }
        upserts.truncate(query.limit);
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
        if result.id.as_str() == "proj:configure" || result.kind == "not_configured" {
            return vec![ActionDescriptor {
                id: ActionId::new("seed_config"),
                label: "Show command".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        if result.kind == "directory"
            || result.kind == "imported"
            || result.primary_action.id.as_str() == "browse"
        {
            let mut actions = vec![
                ActionDescriptor {
                    id: ActionId::new("browse"),
                    label: "Browse".into(),
                    risk: ActionRisk::Safe,
                    confirmation: false,
                },
                ActionDescriptor {
                    id: ActionId::new("open"),
                    label: "Open in Finder".into(),
                    risk: ActionRisk::Safe,
                    confirmation: false,
                },
            ];
            if result.kind == "directory" && result.primary_action.id.as_str() == "import_project" {
                actions.insert(
                    0,
                    ActionDescriptor {
                        id: ActionId::new("import_project"),
                        label: "Import project".into(),
                        risk: ActionRisk::Safe,
                        confirmation: false,
                    },
                );
            }
            return actions;
        }
        if result.primary_action.id.as_str() == "import_project" {
            return vec![ActionDescriptor {
                id: ActionId::new("import_project"),
                label: "Import".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        if result.primary_action.id.as_str() == "remove_project" {
            return vec![ActionDescriptor {
                id: ActionId::new("remove_project"),
                label: "Remove".into(),
                risk: ActionRisk::Confirm,
                confirmation: true,
            }];
        }
        if result.kind == "status"
            || result.kind == "onboarding"
            || result.kind == "unavailable"
            || result.primary_action.id.as_str() == "noop"
        {
            return vec![ActionDescriptor {
                id: ActionId::new("noop"),
                label: "OK".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        let mut actions = vec![ActionDescriptor {
            id: ActionId::new("open"),
            label: "Open".into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        }];
        if result.kind == "project" {
            actions.push(ActionDescriptor {
                id: ActionId::new("browse"),
                label: "Browse".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            });
        }
        actions
    }

    async fn preview(&self, result: &SearchItem) -> Option<String> {
        if result.kind == "directory" || result.id.as_str().starts_with("browse:proj:") {
            let path_part = result
                .subtitle
                .as_deref()?
                .split(" — ")
                .next()
                .unwrap_or("");
            let cancel = CancellationToken::new();
            let roots = self.roots.read().await.clone();
            let directory = self
                .workspace
                .resolve_browse_path(PathBuf::from(path_part), roots, cancel.clone())
                .await
                .ok()?;
            let listing = match self.workspace.list_directory(directory, cancel).await {
                Ok(listing) => listing,
                Err(ProjectWorkspaceError::NotFound(_)) => ProjectDirectoryListing::default(),
                Err(_) => return None,
            };
            return Some(format_projects_directory_preview(listing));
        }
        result
            .subtitle
            .clone()
            .or_else(|| Some(result.title.clone()))
    }

    async fn perform(&self, action: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        match action.action.id.as_str() {
            "noop" => ActionOutcome::Success {
                message: Some("ok".into()),
            },
            "configure" => ActionOutcome::Failed {
                kind: FailureKind::NotConfigured {
                    remediation: "Run: luma config set --projects-root ~/dev".into(),
                },
            },
            "browse" => ActionOutcome::Failed {
                kind: FailureKind::InvalidInput {
                    field: "action".into(),
                    message: "browse is search-driven; use `proj browse <path>`".into(),
                },
            },
            "import_project" => {
                let path_str = action
                    .result
                    .action_payload
                    .as_ref()
                    .and_then(|p| p.get("path"))
                    .and_then(|v| v.as_str())
                    .or_else(|| {
                        action
                            .result
                            .subtitle
                            .as_deref()
                            .map(|s| s.split(" — ").next().unwrap_or(s))
                    })
                    .or_else(|| action.result.id.as_str().strip_prefix("proj:import:"));
                let Some(path_str) = path_str else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "path".into(),
                            message: "missing import path".into(),
                        },
                    };
                };
                match await_unless_cancelled(
                    &cancel,
                    self.workspace
                        .resolve_import_path(PathBuf::from(path_str), cancel.clone()),
                )
                .await
                {
                    None | Some(Err(ProjectWorkspaceError::Cancelled)) => ActionOutcome::Cancelled,
                    Some(Ok(canon)) => ActionOutcome::SettingsMutation {
                        patch: serde_json::json!({
                            "import_project": canon.display().to_string()
                        }),
                    },
                    Some(Err(error)) => ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: error.to_string(),
                        },
                    },
                }
            }
            "remove_project" => {
                let key = action
                    .result
                    .action_payload
                    .as_ref()
                    .and_then(|p| p.get("name"))
                    .and_then(|v| v.as_str())
                    .or_else(|| action.result.id.as_str().strip_prefix("proj:remove:"));
                let Some(key) = key else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "name".into(),
                            message: "missing project name".into(),
                        },
                    };
                };
                ActionOutcome::SettingsMutation {
                    patch: serde_json::json!({ "remove_project": key }),
                }
            }
            "open" => {
                let path_str = action
                    .result
                    .id
                    .as_str()
                    .strip_prefix("browse:proj:")
                    .or_else(|| action.result.id.as_str().strip_prefix("proj:"))
                    .or(action
                        .result
                        .subtitle
                        .as_deref()
                        .map(|s| s.split(" — ").next().unwrap_or(s)));
                let Some(path_str) = path_str else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected proj:<path>".into(),
                        },
                    };
                };
                let path = PathBuf::from(path_str);
                let imported = self.imported.read().await.clone();
                let is_imported = imported.iter().any(|p| p.path == path_str);
                let scope = if is_imported {
                    ProjectOpenScope::ImportedProject
                } else {
                    ProjectOpenScope::ProjectRoots
                };
                let roots = self.roots.read().await.clone();
                let open_path = match await_unless_cancelled(
                    &cancel,
                    self.workspace
                        .resolve_open_path(path, scope, roots, cancel.clone()),
                )
                .await
                {
                    None | Some(Err(ProjectWorkspaceError::Cancelled)) => {
                        return ActionOutcome::Cancelled;
                    }
                    Some(Ok(path)) => path,
                    Some(Err(ProjectWorkspaceError::Denied(reason))) => {
                        return ActionOutcome::Failed {
                            kind: FailureKind::SecurityDenied { reason },
                        };
                    }
                    Some(Err(ProjectWorkspaceError::NotFound(_))) => {
                        return ActionOutcome::Failed {
                            kind: FailureKind::NotFound {
                                entity: path_str.into(),
                            },
                        };
                    }
                    Some(Err(ProjectWorkspaceError::Unavailable(reason))) => {
                        return ActionOutcome::Failed {
                            kind: FailureKind::Unavailable {
                                reason,
                                retryable: true,
                            },
                        };
                    }
                };
                match await_unless_cancelled(&cancel, self.opener.open(&open_path)).await {
                    None => ActionOutcome::Cancelled,
                    Some(Ok(())) => ActionOutcome::Success {
                        message: Some("opened".into()),
                    },
                    Some(Err(error)) => ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: error.to_string(),
                            retryable: true,
                        },
                    },
                }
            }
            other => ActionOutcome::Failed {
                kind: FailureKind::NotFound {
                    entity: other.into(),
                },
            },
        }
    }

    async fn apply_settings(&self, settings: &luma_application::AppSettings) {
        let roots: Vec<PathBuf> = settings.projects_roots.iter().map(PathBuf::from).collect();
        *self.roots.write().await = roots;
        *self.imported.write().await = settings.imported_projects.clone();
    }

    async fn teardown(&self) {
        self.imported.write().await.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_application::{
        FakeOpenPath, FakeProjectWorkspace, ProjectDirectoryEntry, ProjectDirectoryListing,
    };
    use luma_domain::Query;
    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    #[tokio::test]
    async fn browse_dotdot_yields_denied_row() {
        let root = PathBuf::from("/workspace");
        let module = ProjectsModule::with_roots(
            vec![root.clone()],
            Arc::new(FakeOpenPath::new()),
            Arc::new(FakeProjectWorkspace::new()),
        );
        let (tx, mut rx) = mpsc::channel(8);
        let q = Query::parse(format!("proj browse {}/../outside", root.display()), 20);
        module.search(q, tx, CancellationToken::new()).await;
        let ev = rx.recv().await.expect("chunk");
        let Event::ResultsChunk { upserts, .. } = ev else {
            panic!("expected chunk");
        };
        assert_eq!(upserts[0].id, "proj:denied");
    }

    #[tokio::test]
    async fn browse_preserves_path_case_from_rest_raw() {
        let root = PathBuf::from("/workspace");
        let mixed = root.join("MyApp");
        let module = ProjectsModule::with_roots(
            vec![root],
            Arc::new(FakeOpenPath::new()),
            Arc::new(FakeProjectWorkspace::new()),
        );
        let (tx, mut rx) = mpsc::channel(8);
        let path = mixed.display().to_string();
        // Keep case in the query string.
        let q = Query::parse(format!("proj browse {path}"), 20);
        module.search(q, tx, CancellationToken::new()).await;
        let ev = rx.recv().await.expect("chunk");
        let Event::ResultsChunk { upserts, .. } = ev else {
            panic!("expected chunk");
        };
        assert!(
            upserts.iter().all(|u| u.id != "proj:denied"),
            "case-sensitive path under root must browse: {upserts:?}"
        );
    }

    #[tokio::test]
    async fn browse_relative_name_resolves_under_root() {
        let root = PathBuf::from("/workspace");
        let module = ProjectsModule::with_roots(
            vec![root],
            Arc::new(FakeOpenPath::new()),
            Arc::new(FakeProjectWorkspace::new()),
        );
        let (tx, mut rx) = mpsc::channel(8);
        let q = Query::parse("proj browse empty-dir", 20);
        module.search(q, tx, CancellationToken::new()).await;
        let ev = rx.recv().await.expect("chunk");
        let Event::ResultsChunk { upserts, .. } = ev else {
            panic!("expected chunk");
        };
        assert_eq!(
            upserts[0].id, "proj:browse-empty",
            "relative browse under root should list empty folder: {upserts:?}"
        );
        assert_eq!(upserts[0].title, "Empty folder");
    }

    #[tokio::test]
    async fn perform_browse_is_not_success() {
        let module = ProjectsModule::with_roots(
            vec![],
            Arc::new(FakeOpenPath::new()),
            Arc::new(FakeProjectWorkspace::new()),
        );
        let outcome = module
            .perform(
                ActionRequest {
                    result: SearchItem {
                        id: luma_domain::ResultId::new("browse:proj:/tmp"),
                        module_id: ModuleId::new("luma.projects"),
                        title: "x".into(),
                        subtitle: None,
                        kind: "directory".into(),
                        score: 1.0,
                        primary_action: ActionDescriptor {
                            id: ActionId::new("browse"),
                            label: "Browse".into(),
                            risk: ActionRisk::Safe,
                            confirmation: false,
                        },
                        secondary_actions: vec![],
                        ui_intent: None,
                        action_payload: None,
                    },
                    action: ActionDescriptor {
                        id: ActionId::new("browse"),
                        label: "Browse".into(),
                        risk: ActionRisk::Safe,
                        confirmation: false,
                    },
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(outcome, ActionOutcome::Failed { .. }));
    }

    #[tokio::test]
    async fn empty_proj_lists_not_configured() {
        let module = ProjectsModule::with_roots(
            vec![PathBuf::from("/workspace")],
            Arc::new(FakeOpenPath::new()),
            Arc::new(FakeProjectWorkspace::new()),
        );
        let (tx, mut rx) = mpsc::channel(8);
        let q = Query::parse("proj", 20);
        module.search(q, tx, CancellationToken::new()).await;
        let ev = rx.recv().await.expect("chunk");
        let Event::ResultsChunk { upserts, .. } = ev else {
            panic!("expected chunk");
        };
        assert_eq!(upserts[0].id, "proj:not-configured");
        assert_eq!(upserts[0].kind, "not_configured");
    }

    #[tokio::test]
    async fn imported_project_availability_comes_from_workspace_port() {
        let path = PathBuf::from("/workspace/missing-project");
        let workspace = Arc::new(FakeProjectWorkspace::new());
        workspace.mark_missing(path.clone());
        let module = ProjectsModule::with_settings(
            vec![PathBuf::from("/workspace")],
            vec![ImportedProject {
                path: path.display().to_string(),
                name: Some("missing-project".into()),
            }],
            Arc::new(FakeOpenPath::new()),
            workspace,
        );
        let (tx, mut rx) = mpsc::channel(8);
        module
            .search(Query::parse("proj", 20), tx, CancellationToken::new())
            .await;
        let Event::ResultsChunk { upserts, .. } = rx.recv().await.unwrap() else {
            panic!("expected chunk");
        };
        assert_eq!(upserts[0].kind, "unavailable");
        assert_eq!(upserts[0].primary_action_id, "remove_project");
    }

    #[tokio::test]
    async fn proj_add_emits_import_action() {
        let project = PathBuf::from("/workspace/myapp");
        let module = ProjectsModule::with_roots(
            vec![PathBuf::from("/workspace")],
            Arc::new(FakeOpenPath::new()),
            Arc::new(FakeProjectWorkspace::new()),
        );
        let (tx, mut rx) = mpsc::channel(8);
        let q = Query::parse(format!("proj add {}", project.display()), 20);
        module.search(q, tx, CancellationToken::new()).await;
        let ev = rx.recv().await.expect("chunk");
        let Event::ResultsChunk { upserts, .. } = ev else {
            panic!("expected chunk");
        };
        assert_eq!(
            upserts[0].primary_action_id, "import_project",
            "upserts={upserts:?}"
        );
    }

    #[tokio::test]
    async fn proj_add_without_path_shows_usage() {
        let module = ProjectsModule::with_roots(
            vec![PathBuf::from("/workspace")],
            Arc::new(FakeOpenPath::new()),
            Arc::new(FakeProjectWorkspace::new()),
        );
        let (tx, mut rx) = mpsc::channel(8);
        module
            .search(Query::parse("proj add ", 20), tx, CancellationToken::new())
            .await;
        let Event::ResultsChunk { upserts, .. } = rx.recv().await.unwrap() else {
            panic!("expected chunk");
        };
        assert_eq!(upserts[0].id, "proj:import-usage");
        assert!(upserts[0]
            .subtitle
            .as_deref()
            .is_some_and(|text| text.contains("Usage: proj add")));
    }

    #[tokio::test]
    async fn browse_rows_have_real_primary_actions() {
        let root = PathBuf::from("/workspace");
        let project = root.join("myapp");
        let workspace = Arc::new(FakeProjectWorkspace::new());
        workspace.set_listing(
            root.clone(),
            ProjectDirectoryListing {
                entries: vec![ProjectDirectoryEntry {
                    name: "myapp".into(),
                    path: project.clone(),
                    is_directory: true,
                }],
                truncated: false,
            },
        );

        let module = ProjectsModule::with_roots(
            vec![root.clone()],
            Arc::new(FakeOpenPath::new()),
            workspace.clone(),
        );
        let (tx, mut rx) = mpsc::channel(8);
        module
            .search(
                Query::parse(format!("proj browse {}", root.display()), 20),
                tx,
                CancellationToken::new(),
            )
            .await;
        let Event::ResultsChunk { upserts, .. } = rx.recv().await.unwrap() else {
            panic!("expected chunk");
        };
        let candidate = upserts.iter().find(|item| item.title == "myapp/").unwrap();
        assert_eq!(candidate.primary_action_id, "import_project");
        assert!(candidate.ui_intent.is_none());

        let imported_module = ProjectsModule::with_settings(
            vec![root.clone()],
            vec![ImportedProject {
                path: project.display().to_string(),
                name: Some("myapp".into()),
            }],
            Arc::new(FakeOpenPath::new()),
            workspace,
        );
        let (tx, mut rx) = mpsc::channel(8);
        imported_module
            .search(
                Query::parse(format!("proj browse {}", root.display()), 20),
                tx,
                CancellationToken::new(),
            )
            .await;
        let Event::ResultsChunk { upserts, .. } = rx.recv().await.unwrap() else {
            panic!("expected chunk");
        };
        let imported = upserts.iter().find(|item| item.title == "myapp/").unwrap();
        assert_eq!(imported.primary_action_id, "browse");
        assert_eq!(imported.ui_intent, Some(UiIntent::Browse));
    }

    #[tokio::test]
    async fn browse_is_driven_by_the_workspace_fake_not_host_filesystem() {
        let root = PathBuf::from("/workspace");
        let project = root.join("CaseSensitiveProject");
        let workspace = Arc::new(FakeProjectWorkspace::new());
        workspace.set_listing(
            root.clone(),
            ProjectDirectoryListing {
                entries: vec![ProjectDirectoryEntry {
                    name: "CaseSensitiveProject".into(),
                    path: project.clone(),
                    is_directory: true,
                }],
                truncated: false,
            },
        );
        let module = ProjectsModule::with_roots(
            vec![root.clone()],
            Arc::new(FakeOpenPath::new()),
            workspace,
        );
        let (tx, mut rx) = mpsc::channel(8);
        module
            .search(
                Query::parse(format!("proj browse {}", root.display()), 20),
                tx,
                CancellationToken::new(),
            )
            .await;
        let Event::ResultsChunk { upserts, .. } = rx.recv().await.unwrap() else {
            panic!("expected chunk");
        };
        let project_label = project.display().to_string();
        assert!(upserts.iter().any(|item| {
            item.title == "CaseSensitiveProject/"
                && item.subtitle.as_deref() == Some(project_label.as_str())
        }));
    }

    #[tokio::test]
    async fn denied_open_never_reaches_the_open_path_port() {
        let root = PathBuf::from("/workspace");
        let path = root.join("unsafe");
        let workspace = Arc::new(FakeProjectWorkspace::new());
        workspace.fail_open(
            path.clone(),
            ProjectWorkspaceError::Denied("symlink not allowed".into()),
        );
        let opener = Arc::new(FakeOpenPath::new());
        let module = ProjectsModule::with_roots(vec![root], opener.clone(), workspace);
        let action = ActionDescriptor {
            id: ActionId::new("open"),
            label: "Open".into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        };
        let outcome = module
            .perform(
                ActionRequest {
                    result: SearchItem {
                        id: luma_domain::ResultId::new(format!("proj:{}", path.display())),
                        module_id: ModuleId::new("luma.projects"),
                        title: "unsafe".into(),
                        subtitle: Some(path.display().to_string()),
                        kind: "file".into(),
                        score: 1.0,
                        primary_action: action.clone(),
                        secondary_actions: vec![],
                        ui_intent: None,
                        action_payload: None,
                    },
                    action,
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
        assert_eq!(
            opener.open_count.load(std::sync::atomic::Ordering::SeqCst),
            0
        );
    }
}
