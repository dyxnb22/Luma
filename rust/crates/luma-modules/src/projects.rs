use async_trait::async_trait;
use luma_application::{
    validate_import_project_path, ActionOutcome, ActionRequest, ImportedProject, LumaModule,
    ModuleManifest, ModuleState, OpenPathPort, SearchMode, SearchSink, WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{Event, SearchItemDto, UiIntent};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

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
}

impl ProjectsModule {
    pub fn with_roots(roots: Vec<PathBuf>, opener: Arc<dyn OpenPathPort>) -> Self {
        Self::with_settings(roots, Vec::new(), opener)
    }

    pub fn with_settings(
        roots: Vec<PathBuf>,
        imported: Vec<ImportedProject>,
        opener: Arc<dyn OpenPathPort>,
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
        }
    }

    async fn is_imported_path(&self, path: &Path) -> bool {
        let Ok(canon) = path.canonicalize() else {
            return false;
        };
        let canon_str = canon.display().to_string();
        self.imported
            .read()
            .await
            .iter()
            .any(|p| p.path == canon_str)
    }
}

fn resolve_import_path(path: &Path) -> Result<PathBuf, String> {
    validate_import_project_path(path)
}

fn imported_index(imported: &[ImportedProject]) -> Vec<Project> {
    imported
        .iter()
        .map(|p| {
            let path = PathBuf::from(&p.path);
            let missing = !path.exists();
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

fn list_children(dir: &PathBuf, cancel: &CancellationToken) -> Vec<(String, PathBuf, bool)> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in rd.flatten() {
        if cancel.is_cancelled() {
            break;
        }
        let path = entry.path();
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        // Never follow or enumerate symlinks during browse.
        if meta.file_type().is_symlink() {
            continue;
        }
        let is_dir = meta.file_type().is_dir();
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();
        if name.starts_with('.') {
            continue;
        }
        out.push((name, path, is_dir));
    }
    out.sort_by(|a, b| match (b.2, a.2) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.0.to_lowercase().cmp(&b.0.to_lowercase()),
    });
    out
}

fn format_projects_directory_preview(dir: &Path) -> String {
    const MAX: usize = 40;
    let children = list_children(&dir.to_path_buf(), &CancellationToken::new());
    if children.is_empty() {
        return "Empty folder".into();
    }
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    for (name, _, is_dir) in children {
        if is_dir {
            dirs.push(name);
        } else {
            files.push(name);
        }
    }
    let total = dirs.len() + files.len();
    let mut out = format!("{total} item(s):\n");
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
    }
    out.trim_end().to_string()
}

/// Reject `..` components and require the path (after canonicalize when it exists)
/// to sit under at least one configured root.
///
/// Relative paths are tried under each configured root (`proj browse empty-dir`).
#[allow(clippy::ptr_arg)]
fn resolve_under_roots(path: &PathBuf, roots: &[PathBuf]) -> Result<PathBuf, String> {
    for c in path.components() {
        if matches!(c, std::path::Component::ParentDir) {
            return Err("path escapes project roots (..)".into());
        }
    }
    let candidates: Vec<PathBuf> = if path.is_absolute() {
        vec![path.clone()]
    } else {
        roots.iter().map(|r| r.join(path)).collect()
    };
    if candidates.is_empty() {
        return Err("no accessible project roots".into());
    }
    let mut last_err = "path escapes project roots".to_string();
    for candidate in candidates {
        match resolve_candidate_under_roots(&candidate, roots) {
            Ok(resolved) => return Ok(resolved),
            Err(err) => last_err = err,
        }
    }
    Err(last_err)
}

#[allow(clippy::ptr_arg)]
fn resolve_candidate_under_roots(path: &PathBuf, roots: &[PathBuf]) -> Result<PathBuf, String> {
    let root_canons: Vec<PathBuf> = roots.iter().filter_map(|r| r.canonicalize().ok()).collect();
    if root_canons.is_empty() {
        return Err("no accessible project roots".into());
    }
    // Resolve through longest existing ancestor (macOS /var vs /private/var).
    let mut existing = path.clone();
    let mut missing: Vec<std::ffi::OsString> = Vec::new();
    while !existing.as_os_str().is_empty() && !existing.exists() {
        match existing.file_name() {
            Some(name) => {
                missing.push(name.to_os_string());
                existing.pop();
            }
            None => break,
        }
    }
    if !existing.exists() {
        return Err("path has no existing ancestor under project roots".into());
    }
    {
        let Ok(meta) = std::fs::symlink_metadata(&existing) else {
            return Err("cannot stat path".into());
        };
        if meta.file_type().is_symlink() {
            return Err("symlink not allowed under project browse".into());
        }
    }
    let mut resolved = existing
        .canonicalize()
        .map_err(|e| format!("cannot resolve path: {e}"))?;
    if !root_canons.iter().any(|r| resolved.starts_with(r)) {
        return Err("path escapes project roots".into());
    }
    for part in missing.into_iter().rev() {
        if part == ".." {
            return Err("path escapes project roots (..)".into());
        }
        if part == "." {
            continue;
        }
        resolved.push(part);
        if resolved.exists() {
            let Ok(meta) = std::fs::symlink_metadata(&resolved) else {
                return Err("cannot stat path".into());
            };
            if meta.file_type().is_symlink() {
                return Err("symlink not allowed under project browse".into());
            }
            let canon = resolved
                .canonicalize()
                .map_err(|e| format!("cannot resolve path: {e}"))?;
            if !root_canons.iter().any(|r| canon.starts_with(r)) {
                return Err("path escapes project roots".into());
            }
            resolved = canon;
        } else if !root_canons.iter().any(|r| resolved.starts_with(r)) {
            return Err("path escapes project roots".into());
        }
    }
    Ok(resolved)
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
            if let Some(dir) = target {
                let denied_label = dir.display().to_string();
                let Ok(dir) = resolve_under_roots(&dir, &roots) else {
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
                };
                for (name, path, is_dir) in list_children(&dir, &cancel) {
                    if is_dir {
                        let imported = self.is_imported_path(&path).await;
                        upserts.push(SearchItemDto {
                            id: format!("browse:proj:{}", path.display()),
                            module_id: "luma.projects".into(),
                            title: format!("{name}/"),
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
                            title: name,
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
                    id: "proj:browse-empty".into(),
                    module_id: "luma.projects".into(),
                    title: "Empty folder".into(),
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
            match resolve_import_path(&path) {
                Ok(canon) => {
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
                Err(err) => {
                    let _ = sink
                        .send(Event::ResultsChunk {
                            request_id: String::new(),
                            sequence: 1,
                            upserts: vec![SearchItemDto {
                                id: "proj:import-denied".into(),
                                module_id: "luma.projects".into(),
                                title: "Cannot import project".into(),
                                subtitle: Some(err),
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

        let index = imported_index(&imported);
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
            return Some(format_projects_directory_preview(Path::new(path_part)));
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
                match resolve_import_path(Path::new(path_str)) {
                    Ok(canon) => ActionOutcome::SettingsMutation {
                        patch: serde_json::json!({
                            "import_project": canon.display().to_string()
                        }),
                    },
                    Err(err) => ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied { reason: err },
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
                let open_path = if is_imported {
                    match resolve_import_path(&path) {
                        Ok(p) => p,
                        Err(_) => path,
                    }
                } else {
                    let roots = self.roots.read().await.clone();
                    match resolve_under_roots(&path, &roots) {
                        Ok(p) => p,
                        Err(reason) => {
                            return ActionOutcome::Failed {
                                kind: FailureKind::SecurityDenied { reason },
                            };
                        }
                    }
                };
                let Ok(meta) = std::fs::symlink_metadata(&open_path) else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::NotFound {
                            entity: open_path.display().to_string(),
                        },
                    };
                };
                if meta.file_type().is_symlink() {
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "refusing to open symlink".into(),
                        },
                    };
                }
                match self.opener.open(&open_path).await {
                    Ok(()) => ActionOutcome::Success {
                        message: Some("opened".into()),
                    },
                    Err(e) => ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: e.to_string(),
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
    use luma_application::FakeOpenPath;
    use luma_domain::Query;
    use std::os::unix::fs::symlink;
    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    #[test]
    fn resolve_rejects_parent_dir_escape() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let escape = root.join("..").join("outside");
        let err = resolve_under_roots(&escape, &[root]).unwrap_err();
        assert!(err.contains("..") || err.contains("escape"), "{err}");
    }

    #[test]
    fn resolve_rejects_symlink_file_outside() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let secret = outside.path().join("secret.txt");
        std::fs::write(&secret, "leak").unwrap();
        let link = root.path().join("secret.txt");
        symlink(&secret, &link).unwrap();
        let err = resolve_under_roots(&link, &[root.path().to_path_buf()]).unwrap_err();
        assert!(err.contains("symlink") || err.contains("escape"), "{err}");
    }

    #[test]
    fn list_children_skips_symlinks() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("x.md"), "x").unwrap();
        let link = root.path().join("escape-link");
        symlink(outside.path(), &link).unwrap();
        std::fs::create_dir(root.path().join("real")).unwrap();
        let kids = list_children(&root.path().to_path_buf(), &CancellationToken::new());
        assert!(kids.iter().all(|(n, _, _)| n != "escape-link"));
        assert!(kids.iter().any(|(n, _, d)| n == "real" && *d));
    }

    #[tokio::test]
    async fn browse_dotdot_yields_denied_row() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("App")).unwrap();
        std::fs::write(root.path().join("App").join("Cargo.toml"), "").unwrap();
        let module = ProjectsModule::with_roots(
            vec![root.path().to_path_buf()],
            Arc::new(FakeOpenPath::new()),
        );
        let (tx, mut rx) = mpsc::channel(8);
        let q = Query::parse(
            format!("proj browse {}/../outside", root.path().display()),
            20,
        );
        module.search(q, tx, CancellationToken::new()).await;
        let ev = rx.recv().await.expect("chunk");
        let Event::ResultsChunk { upserts, .. } = ev else {
            panic!("expected chunk");
        };
        assert_eq!(upserts[0].id, "proj:denied");
    }

    #[tokio::test]
    async fn browse_preserves_path_case_from_rest_raw() {
        let root = tempfile::tempdir().unwrap();
        let mixed = root.path().join("MyApp");
        std::fs::create_dir(&mixed).unwrap();
        std::fs::write(mixed.join("Cargo.toml"), "").unwrap();
        let module = ProjectsModule::with_roots(
            vec![root.path().to_path_buf()],
            Arc::new(FakeOpenPath::new()),
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
        let root = tempfile::tempdir().unwrap();
        let empty = root.path().join("empty-dir");
        std::fs::create_dir(&empty).unwrap();
        let module = ProjectsModule::with_roots(
            vec![root.path().to_path_buf()],
            Arc::new(FakeOpenPath::new()),
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
        let module = ProjectsModule::with_roots(vec![], Arc::new(FakeOpenPath::new()));
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
        let root = tempfile::tempdir().unwrap();
        let module = ProjectsModule::with_roots(
            vec![root.path().to_path_buf()],
            Arc::new(FakeOpenPath::new()),
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
    async fn proj_add_emits_import_action() {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("myapp");
        std::fs::create_dir(&project).unwrap();
        let module = ProjectsModule::with_roots(
            vec![root.path().to_path_buf()],
            Arc::new(FakeOpenPath::new()),
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
        let root = tempfile::tempdir().unwrap();
        let module = ProjectsModule::with_roots(
            vec![root.path().to_path_buf()],
            Arc::new(FakeOpenPath::new()),
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
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("myapp");
        std::fs::create_dir(&project).unwrap();

        let module = ProjectsModule::with_roots(
            vec![root.path().to_path_buf()],
            Arc::new(FakeOpenPath::new()),
        );
        let (tx, mut rx) = mpsc::channel(8);
        module
            .search(
                Query::parse(format!("proj browse {}", root.path().display()), 20),
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
            vec![root.path().to_path_buf()],
            vec![ImportedProject {
                path: project.canonicalize().unwrap().display().to_string(),
                name: Some("myapp".into()),
            }],
            Arc::new(FakeOpenPath::new()),
        );
        let (tx, mut rx) = mpsc::channel(8);
        imported_module
            .search(
                Query::parse(format!("proj browse {}", root.path().display()), 20),
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
}
