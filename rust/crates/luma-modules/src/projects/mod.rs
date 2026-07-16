use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, ImportedProject, LumaModule, ModuleManifest, ModuleState,
    OpenPathPort, ProjectWorkspacePort, SearchMode, SearchSink, WarmupContext,
};
use luma_domain::{ActionDescriptor, ModuleId, Query, SearchItem};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

mod actions;
mod preview;
mod search;

#[derive(Clone)]
pub(super) struct Project {
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
        Self::with_deps(roots, Vec::new(), opener, workspace)
    }

    pub fn with_deps(
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
        self.search_projects(query, sink, cancel).await;
    }

    async fn actions(&self, result: &SearchItem) -> Vec<ActionDescriptor> {
        self.module_actions(result).await
    }

    async fn preview(&self, result: &SearchItem) -> Option<String> {
        self.module_preview(result).await
    }

    async fn perform(&self, action: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        self.module_perform(action, cancel).await
    }

    async fn apply_settings(&self, settings: &luma_application::AppSettings) {
        self.apply_module_settings(settings).await;
    }

    async fn teardown(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_application::{
        FakeOpenPath, FakeProjectWorkspace, ProjectDirectoryEntry, ProjectDirectoryListing,
        ProjectWorkspaceError,
    };
    use luma_domain::{ActionId, ActionRisk, FailureKind, Query};
    use luma_protocol::{Event, SearchItemDto, UiIntent};
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
        let module = ProjectsModule::with_deps(
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

        let imported_module = ProjectsModule::with_deps(
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
