use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, LumaModule, ModuleManifest, ModuleState, OpenPathPort,
    SearchMode, SearchSink, WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{Event, SearchItemDto};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
struct Project {
    name: String,
    path: PathBuf,
}

pub struct ProjectsModule {
    manifest: ModuleManifest,
    roots: Arc<RwLock<Vec<PathBuf>>>,
    index: Arc<RwLock<Vec<Project>>>,
    opener: Arc<dyn OpenPathPort>,
}

impl ProjectsModule {
    pub fn with_roots(roots: Vec<PathBuf>, opener: Arc<dyn OpenPathPort>) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.projects"),
                display_name: "Projects".into(),
                triggers: vec!["p".into(), "proj".into(), "project".into()],
                default_enabled: false,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
                workbench: luma_application::WorkbenchMeta {
                    glyph: Some("P".into()),
                    suggested_query: Some("proj ".into()),
                    empty_hint: Some("proj · proj browse <path>".into()),
                    supports_browse: true,
                },
            },
            roots: Arc::new(RwLock::new(roots)),
            index: Arc::new(RwLock::new(Vec::new())),
            opener,
        }
    }

    /// Replace scan roots and refresh the project index.
    pub async fn set_roots(&self, roots: Vec<PathBuf>) {
        *self.roots.write().await = roots.clone();
        *self.index.write().await = scan_projects(&roots);
    }
}

fn scan_projects(roots: &[PathBuf]) -> Vec<Project> {
    let mut out = Vec::new();
    for root in roots {
        let Ok(rd) = std::fs::read_dir(root) else {
            continue;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.join(".git").exists()
                || path.join("Package.swift").exists()
                || path.join("Cargo.toml").exists()
            {
                let name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("project")
                    .to_string();
                out.push(Project { name, path });
            }
        }
    }
    out
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

    async fn warmup(&self, ctx: WarmupContext) -> ModuleState {
        if ctx.cancel.is_cancelled() {
            return ModuleState::Cold;
        }
        let roots = self.roots.read().await.clone();
        if roots.is_empty() {
            return ModuleState::Cold;
        }
        let cancel = ctx.cancel.clone();
        let handle = tokio::task::spawn_blocking(move || scan_projects(&roots));
        let abort = handle.abort_handle();
        let idx = tokio::select! {
            _ = cancel.cancelled() => {
                abort.abort();
                return ModuleState::Cold;
            }
            result = handle => result.unwrap_or_default(),
        };
        *self.index.write().await = idx;
        ModuleState::Ready
    }

    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        let roots = self.roots.read().await.clone();
        if roots.is_empty() {
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![SearchItemDto {
                        id: "proj:configure".into(),
                        module_id: "luma.projects".into(),
                        title: "Add a project scan root".into(),
                        subtitle: Some(
                            "NotConfigured — run: luma config set --projects-root ~/dev".into(),
                        ),
                        kind: "not_configured".into(),
                        score: 0.0,
                        primary_action_id: "configure".into(),
                        primary_action_label: "Configure".into(),
                        ..Default::default()
                    }],
                    removed_ids: vec![],
                })
                .await;
            return;
        }
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
                        upserts.push(SearchItemDto {
                            id: format!("browse:proj:{}", path.display()),
                            module_id: "luma.projects".into(),
                            title: format!("{name}/"),
                            subtitle: Some(path.display().to_string()),
                            kind: "directory".into(),
                            score: 80.0,
                            primary_action_id: "browse".into(),
                            primary_action_label: "Browse".into(),
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

        let needle = rest_norm;
        let index = self.index.read().await.clone();
        let mut upserts = Vec::new();
        for p in index {
            if cancel.is_cancelled() {
                return;
            }
            if needle.is_empty() || p.name.to_lowercase().contains(&needle) {
                upserts.push(SearchItemDto {
                    id: format!("proj:{}", p.path.display()),
                    module_id: "luma.projects".into(),
                    title: p.name,
                    subtitle: Some(p.path.display().to_string()),
                    kind: "project".into(),
                    score: 65.0,
                    primary_action_id: "open".into(),
                    primary_action_label: "Open".into(),
                    ..Default::default()
                });
            }
        }
        if upserts.is_empty() {
            let (title, subtitle) = if needle.is_empty() {
                (
                    "No projects found".into(),
                    "Check roots or run: luma config set --projects-root ~/dev".into(),
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
        if result.id.as_str() == "proj:configure" {
            return vec![ActionDescriptor {
                id: ActionId::new("configure"),
                label: "Configure".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        if result.kind == "directory" || result.primary_action.id.as_str() == "browse" {
            return vec![
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
        }
        if result.kind == "status"
            || result.kind == "not_configured"
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
            "open" => {
                let path_str = action
                    .result
                    .id
                    .as_str()
                    .strip_prefix("browse:proj:")
                    .or_else(|| action.result.id.as_str().strip_prefix("proj:"))
                    .or(action.result.subtitle.as_deref());
                let Some(path) = path_str.map(PathBuf::from) else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected proj:<path>".into(),
                        },
                    };
                };
                let roots = self.roots.read().await.clone();
                let Ok(path) = resolve_under_roots(&path, &roots) else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "path not under scan roots".into(),
                        },
                    };
                };
                let Ok(meta) = std::fs::symlink_metadata(&path) else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::NotFound {
                            entity: path.display().to_string(),
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
                // Re-resolve after metadata check to shrink swap window.
                let Ok(path) = resolve_under_roots(&path, &roots) else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "path not under scan roots".into(),
                        },
                    };
                };
                match self.opener.open(&path).await {
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
        self.set_roots(roots).await;
    }

    async fn teardown(&self) {
        self.index.write().await.clear();
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
}
