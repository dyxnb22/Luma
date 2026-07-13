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
            },
            roots: Arc::new(RwLock::new(roots)),
            index: Arc::new(RwLock::new(Vec::new())),
            opener,
        }
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
        let idx = tokio::task::spawn_blocking(move || scan_projects(&roots))
            .await
            .unwrap_or_default();
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
                        subtitle: Some("NotConfigured".into()),
                        kind: "onboarding".into(),
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
        let needle = query
            .normalized
            .split_once(|c: char| c.is_whitespace())
            .map(|(_, r)| r.trim().to_string())
            .unwrap_or_default();
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
        if !upserts.is_empty() {
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

    async fn actions(&self, result: &SearchItem) -> Vec<ActionDescriptor> {
        if result.id.as_str() == "proj:configure" {
            return vec![ActionDescriptor {
                id: ActionId::new("configure"),
                label: "Configure".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        vec![ActionDescriptor {
            id: ActionId::new("open"),
            label: "Open".into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        }]
    }

    async fn perform(&self, action: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        match action.action.id.as_str() {
            "configure" => ActionOutcome::Failed {
                kind: FailureKind::NotConfigured {
                    remediation: "Configure project scan roots in settings (coming soon)".into(),
                },
            },
            "open" => {
                let Some(path) = action
                    .result
                    .id
                    .as_str()
                    .strip_prefix("proj:")
                    .map(PathBuf::from)
                else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected proj:<path>".into(),
                        },
                    };
                };
                // Containment: must be under a configured root.
                let roots = self.roots.read().await.clone();
                let ok = roots.iter().any(|r| path.starts_with(r));
                if !ok {
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "path not under scan roots".into(),
                        },
                    };
                }
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

    async fn teardown(&self) {
        self.index.write().await.clear();
    }
}
