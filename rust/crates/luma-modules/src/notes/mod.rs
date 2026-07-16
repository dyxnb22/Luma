use crate::cancel::await_unless_cancelled;
use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, ClockPort, FakeNotesWorkspace, FakePasteboard, FixedClock,
    LumaModule, MarkdownWatchPort, MemoryNotesIndex, ModuleManifest, ModuleState,
    NotesIndexRepository, NotesWorkspaceError, NotesWorkspacePort, OpenPathPort, PasteboardPort,
    SearchMode, SearchSink, WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

mod preview;
mod rows;
mod search;
mod watch;

pub struct NotesModule {
    manifest: ModuleManifest,
    root: Arc<RwLock<Option<PathBuf>>>,
    index: Arc<dyn NotesIndexRepository>,
    /// Last applied exclude patterns (skip rebuild when unchanged).
    exclude_patterns: Arc<RwLock<Vec<String>>>,
    /// Bumped on each rebuild so late watch callbacks can be ignored if needed.
    index_generation: Arc<RwLock<u64>>,
    watch_cancel: Mutex<Option<CancellationToken>>,
    watch_handle: Mutex<Option<JoinHandle<()>>>,
    watch_warning: Arc<Mutex<Option<String>>>,
    opener: Arc<dyn OpenPathPort>,
    pasteboard: Arc<dyn PasteboardPort>,
    watcher: Arc<dyn MarkdownWatchPort>,
    clock: Arc<dyn ClockPort>,
    workspace: Arc<dyn NotesWorkspacePort>,
}

/// Platform-facing services supplied by the composition root. Grouping these keeps the module
/// constructor readable while preserving explicit port injection for both production and tests.
pub struct NotesServices {
    pub clock: Arc<dyn ClockPort>,
    pub workspace: Arc<dyn NotesWorkspacePort>,
}

impl NotesModule {
    fn workspace_failure(error: NotesWorkspaceError) -> ActionOutcome {
        match error {
            NotesWorkspaceError::Cancelled => ActionOutcome::Cancelled,
            NotesWorkspaceError::OutsideWorkspace => ActionOutcome::Failed {
                kind: FailureKind::SecurityDenied {
                    reason: "notes path is outside the configured workspace".into(),
                },
            },
            NotesWorkspaceError::InvalidItem => ActionOutcome::Failed {
                kind: FailureKind::SecurityDenied {
                    reason: "refusing a symlink or non-regular notes item".into(),
                },
            },
            NotesWorkspaceError::NotFound => ActionOutcome::Failed {
                kind: FailureKind::NotFound {
                    entity: "note".into(),
                },
            },
            NotesWorkspaceError::AlreadyExists => ActionOutcome::Failed {
                kind: FailureKind::InvalidInput {
                    field: "path".into(),
                    message: "note already exists".into(),
                },
            },
            NotesWorkspaceError::RootUnavailable | NotesWorkspaceError::Unavailable => {
                ActionOutcome::Failed {
                    kind: FailureKind::Unavailable {
                        reason: error.to_string(),
                        retryable: true,
                    },
                }
            }
        }
    }

    /// Constructor used by the composition root with an injected local-clock adapter.
    pub fn with_root(
        root: Option<PathBuf>,
        opener: Arc<dyn OpenPathPort>,
        watcher: Arc<dyn MarkdownWatchPort>,
        index: Arc<dyn NotesIndexRepository>,
        pasteboard: Arc<dyn PasteboardPort>,
        services: NotesServices,
        exclude_patterns: Vec<String>,
    ) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.notes"),
                display_name: "Notes".into(),
                triggers: vec!["n".into(), "note".into(), "notes".into()],
                default_enabled: true,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
                workbench: luma_application::WorkbenchMeta {
                    glyph: Some("N".into()),
                    // Directory-first: Hub / `n ` open the notes tree; `n recent` for flat recent.
                    suggested_query: Some("n ".into()),
                    empty_hint: Some(
                        "n ␠ browse tree · n <query> search · n recent · n issues".into(),
                    ),
                    supports_browse: true,
                },
            },
            root: Arc::new(RwLock::new(root)),
            index,
            exclude_patterns: Arc::new(RwLock::new(exclude_patterns)),
            index_generation: Arc::new(RwLock::new(0)),
            watch_cancel: Mutex::new(None),
            watch_handle: Mutex::new(None),
            watch_warning: Arc::new(Mutex::new(None)),
            opener,
            pasteboard,
            watcher,
            clock: services.clock,
            workspace: services.workspace,
        }
    }

    /// Test-only: Fake opener that never opens paths but reports success.
    pub fn with_root_for_tests(root: Option<PathBuf>) -> Self {
        use luma_application::FakeOpenPath;
        Self::with_root_and_opener(root, Arc::new(FakeOpenPath::new()))
    }

    /// Test helper: real opener mock + no-op markdown watcher + memory index.
    pub fn with_root_and_opener(root: Option<PathBuf>, opener: Arc<dyn OpenPathPort>) -> Self {
        Self::with_root(
            root,
            opener,
            Arc::new(watch::NullMarkdownWatcher),
            Arc::new(MemoryNotesIndex::new()),
            Arc::new(FakePasteboard::new()),
            NotesServices {
                clock: Arc::new(FixedClock {
                    ymd: "2026-07-13".into(),
                    now: String::new(),
                }),
                workspace: Arc::new(FakeNotesWorkspace::new()),
            },
            Vec::new(),
        )
    }

    /// Test helper with a custom markdown watcher.
    pub fn with_root_watcher_for_tests(
        root: Option<PathBuf>,
        watcher: Arc<dyn MarkdownWatchPort>,
    ) -> Self {
        use luma_application::FakeOpenPath;
        Self::with_root(
            root,
            Arc::new(FakeOpenPath::new()),
            watcher,
            Arc::new(MemoryNotesIndex::new()),
            Arc::new(FakePasteboard::new()),
            NotesServices {
                clock: Arc::new(FixedClock {
                    ymd: "2026-07-13".into(),
                    now: String::new(),
                }),
                workspace: Arc::new(FakeNotesWorkspace::new()),
            },
            Vec::new(),
        )
    }
}

#[async_trait]
impl LumaModule for NotesModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, ctx: WarmupContext) -> ModuleState {
        let root = self.root.read().await.clone();
        if let Some(root) = root {
            if ctx.cancel.is_cancelled() {
                return ModuleState::Failed("notes warmup cancelled".into());
            }
            let (flag, bridge) = Self::scan_cancel_bridge(&ctx.cancel);
            let result = self.rebuild_index(&root, Some(flag)).await;
            if let Some(bridge) = bridge {
                bridge.abort();
            }
            match result {
                Ok(()) => {
                    if !ctx.cancel.is_cancelled() {
                        self.start_watch(ctx.cancel).await;
                    }
                    ModuleState::Ready
                }
                Err(e) => ModuleState::Failed(e.to_string()),
            }
        } else {
            ModuleState::Cold
        }
    }

    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        NotesModule::search(self, query, sink, cancel).await
    }

    async fn actions(&self, result: &SearchItem) -> Vec<ActionDescriptor> {
        if result.id.as_str() == "notes:configure"
            || result.kind == "not_configured"
            || result.primary_action.id.as_str() == "configure"
            || result.primary_action.id.as_str() == "seed_config"
        {
            return vec![ActionDescriptor {
                id: ActionId::new("seed_config"),
                label: "Show command".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        if result.kind == "directory" || result.primary_action.id.as_str() == "browse" {
            return vec![ActionDescriptor {
                id: ActionId::new("browse"),
                label: "Browse".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        if result.kind == "create" || result.primary_action.id.as_str() == "create" {
            return vec![ActionDescriptor {
                id: ActionId::new("create"),
                label: "Create".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        if result.kind == "status" || result.kind == "onboarding" || result.kind == "unavailable" {
            if result.primary_action.id.as_str() == "list_issues" {
                return vec![ActionDescriptor {
                    id: ActionId::new("list_issues"),
                    label: "View issues".into(),
                    risk: ActionRisk::Safe,
                    confirmation: false,
                }];
            }
            return vec![ActionDescriptor {
                id: ActionId::new("noop"),
                label: "OK".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        vec![
            ActionDescriptor {
                id: ActionId::new("open"),
                label: "Open".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            ActionDescriptor {
                id: ActionId::new("copy_path"),
                label: "Copy Path".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
        ]
    }

    async fn preview(&self, result: &SearchItem) -> Option<String> {
        NotesModule::preview(self, result).await
    }

    async fn perform(&self, action: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        let root = self.root.read().await.clone();
        match action.action.id.as_str() {
            "restart_watch" => {
                let Some(_root) = root else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::NotConfigured {
                            remediation: "Run: luma config set --notes-root ~/Notes".into(),
                        },
                    };
                };
                self.start_watch(CancellationToken::new()).await;
                ActionOutcome::Success {
                    message: Some("restarted notes file watcher".into()),
                }
            }
            "configure" => ActionOutcome::Failed {
                kind: FailureKind::NotConfigured {
                    remediation: "Run: luma config set --notes-root ~/Notes".into(),
                },
            },
            "browse" => ActionOutcome::Failed {
                kind: FailureKind::InvalidInput {
                    field: "action".into(),
                    message: "browse is search-driven; use `n browse <path>`".into(),
                },
            },
            "list_issues" => ActionOutcome::Failed {
                kind: FailureKind::InvalidInput {
                    field: "action".into(),
                    message: "list_issues is search-driven; use `n issues`".into(),
                },
            },
            "create" => {
                let Some(root) = root else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::NotConfigured {
                            remediation: "Run: luma config set --notes-root ~/Notes".into(),
                        },
                    };
                };
                let raw = action
                    .result
                    .id
                    .as_str()
                    .strip_prefix("note:")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("Inbox/new.md"));
                let is_daily = raw
                    .parent()
                    .and_then(|parent| parent.file_name())
                    .is_some_and(|name| name == "Daily");
                let body = if is_daily {
                    let date = raw
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .unwrap_or("today");
                    format!("# {date}\n\n")
                } else {
                    "# New note\n".into()
                };
                let path = match self
                    .workspace
                    .create_note(root.clone(), raw, body, cancel.clone())
                    .await
                {
                    Ok(path) => path.path,
                    Err(error) => return Self::workspace_failure(error),
                };
                self.rebuild_index(&root, None).await.ok();
                // Durable create committed — do not report Cancelled after this point.
                match await_unless_cancelled(&cancel, self.opener.open(&path)).await {
                    None => ActionOutcome::Success {
                        message: Some(format!("created {}; open cancelled", path.display())),
                    },
                    Some(Ok(())) => ActionOutcome::Success {
                        message: Some(format!("created {}", path.display())),
                    },
                    Some(Err(err)) => ActionOutcome::Success {
                        message: Some(format!("created {}; open failed: {err}", path.display())),
                    },
                }
            }
            "open" => {
                let Some(root) = root else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::NotConfigured {
                            remediation: "Set notes_root first".into(),
                        },
                    };
                };
                let Some(raw) = action
                    .result
                    .id
                    .as_str()
                    .strip_prefix("note:")
                    .map(PathBuf::from)
                else {
                    let path = match self
                        .workspace
                        .prepare_open(root.clone(), root.clone(), true, cancel.clone())
                        .await
                    {
                        Ok(path) => path.path,
                        Err(error) => return Self::workspace_failure(error),
                    };
                    // Opening the vetted workspace root directory.
                    return match await_unless_cancelled(&cancel, self.opener.open(&path)).await {
                        None => ActionOutcome::Cancelled,
                        Some(Ok(())) => ActionOutcome::Success {
                            message: Some("opened notes workspace".into()),
                        },
                        Some(Err(err)) => ActionOutcome::Failed {
                            kind: FailureKind::Unavailable {
                                reason: format!("open failed: {err}"),
                                retryable: true,
                            },
                        },
                    };
                };
                let path = match self
                    .workspace
                    .prepare_open(root.clone(), raw, false, cancel.clone())
                    .await
                {
                    Ok(path) => path.path,
                    Err(error) => return Self::workspace_failure(error),
                };
                match await_unless_cancelled(&cancel, self.opener.open(&path)).await {
                    None => ActionOutcome::Cancelled,
                    Some(Ok(())) => ActionOutcome::Success {
                        message: Some("opened".into()),
                    },
                    Some(Err(err)) => ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: format!("open failed: {err}"),
                            retryable: true,
                        },
                    },
                }
            }
            "noop" => ActionOutcome::Success {
                message: Some("ok".into()),
            },
            "copy_path" => {
                let Some(root) = root else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::NotConfigured {
                            remediation: "Set notes_root first".into(),
                        },
                    };
                };
                let Some(raw) = action
                    .result
                    .id
                    .as_str()
                    .strip_prefix("note:")
                    .map(PathBuf::from)
                else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "id".into(),
                            message: "copy_path requires a note result".into(),
                        },
                    };
                };
                let path = match self
                    .workspace
                    .prepare_open(root.clone(), raw, false, cancel.clone())
                    .await
                {
                    Ok(path) => path.path,
                    Err(error) => return Self::workspace_failure(error),
                };
                match await_unless_cancelled(
                    &cancel,
                    self.pasteboard.write_text(&path.display().to_string()),
                )
                .await
                {
                    None => ActionOutcome::Cancelled,
                    Some(Ok(())) => ActionOutcome::Success {
                        message: Some(format!("copied {}", path.display())),
                    },
                    Some(Err(err)) => ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: format!("copy failed: {err}"),
                            retryable: true,
                        },
                    },
                }
            }
            other => ActionOutcome::Failed {
                kind: FailureKind::NotFound {
                    entity: format!("action:{other}"),
                },
            },
        }
    }

    async fn apply_settings(&self, settings: &luma_application::AppSettings) {
        let _ = self
            .index
            .set_scan_exclude_patterns(settings.notes_exclude_patterns.clone());
        let new_root = settings.notes_root.as_ref().map(PathBuf::from);
        let current = self.root.read().await.clone();
        if current != new_root {
            *self.exclude_patterns.write().await = settings.notes_exclude_patterns.clone();
            self.set_root(new_root.clone()).await;
            if new_root.is_some() {
                // Watch until next teardown/set_root (engine session cancel not available here).
                self.start_watch(CancellationToken::new()).await;
            }
        } else if let Some(root) = current {
            let excludes_changed = {
                let mut prev = self.exclude_patterns.write().await;
                if *prev == settings.notes_exclude_patterns {
                    false
                } else {
                    *prev = settings.notes_exclude_patterns.clone();
                    true
                }
            };
            if !excludes_changed {
                return;
            }
            // Exclude patterns changed under the same root → rebuild in background
            // so Settings apply does not stall the session on large vaults.
            let index = self.index.clone();
            let generation = self.index_generation.clone();
            let watch_warning = self.watch_warning.clone();
            tokio::spawn(async move {
                const MAX_ATTEMPTS: u32 = 4;
                for attempt in 0..MAX_ATTEMPTS {
                    let docs = match index.document_count() {
                        Ok(n) => n,
                        Err(err) => {
                            *watch_warning.lock().await = Some(format!(
                                "Notes exclude rebuild failed ({err}); run `n check`"
                            ));
                            return;
                        }
                    };
                    let fts = index.fts_count().unwrap_or(0);
                    let need_full = docs == 0 || (docs > 0 && (fts == 0 || fts != docs));
                    let index_for_scan = index.clone();
                    let root_for_scan = root.clone();
                    let report = tokio::task::spawn_blocking(move || {
                        if need_full {
                            index_for_scan.full_scan(&root_for_scan, None)
                        } else {
                            index_for_scan.incremental_check(&root_for_scan, None)
                        }
                    })
                    .await;
                    match report {
                        Ok(Ok(report)) if !report.cancelled => {
                            let mut gen = generation.write().await;
                            *gen = gen.saturating_add(1);
                            // Clear only exclude-rebuild warnings so watcher warnings survive.
                            let mut warn = watch_warning.lock().await;
                            if warn
                                .as_deref()
                                .is_some_and(|m| m.starts_with("Notes exclude rebuild"))
                            {
                                *warn = None;
                            }
                            return;
                        }
                        Ok(Ok(_)) => {
                            *watch_warning.lock().await =
                                Some("Notes exclude rebuild cancelled; run `n check`".into());
                            return;
                        }
                        Ok(Err(err)) => {
                            let msg = err.to_string();
                            let busy = msg.contains("already running") || msg.contains("Busy");
                            if busy && attempt + 1 < MAX_ATTEMPTS {
                                tokio::time::sleep(Duration::from_millis(
                                    150 * (attempt + 1) as u64,
                                ))
                                .await;
                                continue;
                            }
                            *watch_warning.lock().await = Some(format!(
                                "Notes exclude rebuild failed ({msg}); run `n check`"
                            ));
                            return;
                        }
                        Err(err) => {
                            *watch_warning.lock().await = Some(format!(
                                "Notes exclude rebuild failed ({err}); run `n check`"
                            ));
                            return;
                        }
                    }
                }
            });
        }
    }

    async fn teardown(&self) {
        self.stop_watch().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_application::{
        FakeOpenPath, MemoryNotesIndex, NotesDirectoryEntry, NotesDirectoryEntryKind,
        NotesDirectoryListing, NotesWorkspacePath, NotesWorkspacePreview,
    };
    use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ResultId, SearchItem};
    use luma_protocol::Event;
    use std::path::Path;
    use std::time::Duration;
    use tokio::sync::mpsc;

    fn test_module(
        root: Option<PathBuf>,
        opener: Arc<dyn OpenPathPort>,
        index: Arc<dyn NotesIndexRepository>,
        workspace: Arc<FakeNotesWorkspace>,
    ) -> NotesModule {
        NotesModule::with_root(
            root,
            opener,
            Arc::new(watch::NullMarkdownWatcher),
            index,
            Arc::new(FakePasteboard::new()),
            NotesServices {
                clock: Arc::new(FixedClock {
                    ymd: "2026-07-13".into(),
                    now: String::new(),
                }),
                workspace,
            },
            Vec::new(),
        )
    }

    fn listing_entry(
        root: &Path,
        name: &str,
        kind: NotesDirectoryEntryKind,
    ) -> NotesDirectoryEntry {
        NotesDirectoryEntry {
            name: name.into(),
            path: NotesWorkspacePath {
                path: root.join(name),
                relative_path: name.into(),
            },
            kind,
        }
    }

    fn create_request(path: &Path) -> ActionRequest {
        ActionRequest {
            result: SearchItem {
                id: ResultId::new(format!("note:{}", path.display())),
                module_id: ModuleId::new("luma.notes"),
                title: "x".into(),
                subtitle: None,
                kind: "create".into(),
                score: 1.0,
                primary_action: ActionDescriptor {
                    id: ActionId::new("create"),
                    label: "Create".into(),
                    risk: ActionRisk::Safe,
                    confirmation: false,
                },
                secondary_actions: vec![],
                ui_intent: None,
                action_payload: None,
            },
            action: ActionDescriptor {
                id: ActionId::new("create"),
                label: "Create".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            confirmation: false,
        }
    }

    fn open_request(path: &Path) -> ActionRequest {
        let mut req = create_request(path);
        req.action.id = ActionId::new("open");
        req.result.primary_action.id = ActionId::new("open");
        req.result.kind = "note".into();
        req
    }

    #[tokio::test]
    async fn not_configured_is_not_empty() {
        let m = NotesModule::with_root_for_tests(None);
        let (tx, mut rx) = mpsc::channel(2);
        m.search(Query::parse("n ", 10), tx, CancellationToken::new())
            .await;
        let ev = rx.recv().await.unwrap();
        match ev {
            Event::ResultsChunk { upserts, .. } => {
                assert_eq!(upserts[0].kind, "not_configured");
                assert_eq!(upserts[0].primary_action_id, "seed_config");
                assert!(
                    upserts[0]
                        .subtitle
                        .as_deref()
                        .is_some_and(|s| s.contains("luma config set --notes-root")),
                    "subtitle should carry CLI: {:?}",
                    upserts[0].subtitle
                );
            }
            other => panic!("{other:?}"),
        }
    }

    #[tokio::test]
    async fn search_memory_index() {
        let index = Arc::new(MemoryNotesIndex::new());
        index.insert_document("hello.md", "hi", "# hi");
        let m = test_module(
            Some(PathBuf::from("/notes")),
            Arc::new(FakeOpenPath::new()),
            index,
            Arc::new(FakeNotesWorkspace::new()),
        );
        let (tx, mut rx) = mpsc::channel(2);
        m.search(Query::parse("n hi", 10), tx, CancellationToken::new())
            .await;
        let ev = rx.recv().await.unwrap();
        match ev {
            Event::ResultsChunk { upserts, .. } => {
                assert!(
                    upserts.iter().any(|u| u.title == "hi" && u.kind == "note"),
                    "{upserts:?}"
                );
            }
            other => panic!("{other:?}"),
        }
    }

    #[tokio::test]
    async fn create_rejects_workspace_escape_without_host_io() {
        let root = PathBuf::from("/notes");
        let fake = Arc::new(FakeOpenPath::new());
        let workspace = Arc::new(FakeNotesWorkspace::new());
        let m = test_module(
            Some(root),
            fake,
            Arc::new(MemoryNotesIndex::new()),
            workspace.clone(),
        );
        let req = create_request(&PathBuf::from("../escaped.md"));
        // Use relative escape via result id
        let req = ActionRequest {
            result: SearchItem {
                id: ResultId::new("note:../escaped.md"),
                ..req.result
            },
            ..req
        };
        let out = m.perform(req, CancellationToken::new()).await;
        match out {
            ActionOutcome::Failed {
                kind: FailureKind::SecurityDenied { .. },
            } => {}
            other => panic!("expected security deny, got {other:?}"),
        }
        assert!(workspace.created_notes().is_empty());
    }

    #[tokio::test]
    async fn create_success_even_if_open_fails() {
        let root = PathBuf::from("/notes");
        let fake = Arc::new(FakeOpenPath::with_failure());
        let workspace = Arc::new(FakeNotesWorkspace::new());
        let m = test_module(
            Some(root.clone()),
            fake.clone(),
            Arc::new(MemoryNotesIndex::new()),
            workspace.clone(),
        );
        let path = root.join("Inbox").join("a.md");
        let out = m
            .perform(create_request(&path), CancellationToken::new())
            .await;
        match out {
            ActionOutcome::Success { message } => {
                let msg = message.unwrap();
                assert!(msg.contains("created"), "{msg}");
                assert!(msg.contains("open failed"), "{msg}");
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(
            workspace.created_notes(),
            vec![(path, "# New note\n".into())]
        );
        assert_eq!(fake.open_count.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn open_failure_is_not_success() {
        let root = PathBuf::from("/notes");
        let path = root.join("a.md");
        let fake = Arc::new(FakeOpenPath::with_failure());
        let workspace = Arc::new(FakeNotesWorkspace::new());
        workspace.mark_existing(path.clone());
        let m = test_module(
            Some(root),
            fake,
            Arc::new(MemoryNotesIndex::new()),
            workspace,
        );
        let out = m
            .perform(open_request(&path), CancellationToken::new())
            .await;
        match out {
            ActionOutcome::Failed {
                kind: FailureKind::Unavailable { .. },
            } => {}
            other => panic!("open failure must not be Success: {other:?}"),
        }
    }

    #[tokio::test]
    async fn create_under_inbox_ok() {
        let root = PathBuf::from("/notes");
        let fake = Arc::new(FakeOpenPath::new());
        let workspace = Arc::new(FakeNotesWorkspace::new());
        let m = test_module(
            Some(root.clone()),
            fake,
            Arc::new(MemoryNotesIndex::new()),
            workspace.clone(),
        );
        let req = ActionRequest {
            result: SearchItem {
                id: ResultId::new("note:Inbox/hello.md"),
                module_id: ModuleId::new("luma.notes"),
                title: "x".into(),
                subtitle: None,
                kind: "create".into(),
                score: 1.0,
                primary_action: ActionDescriptor {
                    id: ActionId::new("create"),
                    label: "Create".into(),
                    risk: ActionRisk::Safe,
                    confirmation: false,
                },
                secondary_actions: vec![],
                ui_intent: None,
                action_payload: None,
            },
            action: ActionDescriptor {
                id: ActionId::new("create"),
                label: "Create".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            confirmation: false,
        };
        let out = m.perform(req, CancellationToken::new()).await;
        assert!(matches!(out, ActionOutcome::Success { .. }), "{out:?}");
        assert_eq!(
            workspace.created_notes(),
            vec![(root.join("Inbox/hello.md"), "# New note\n".into())]
        );
    }

    #[tokio::test]
    async fn browse_root_lists_children() {
        let root = PathBuf::from("/notes");
        let workspace = Arc::new(FakeNotesWorkspace::new());
        workspace.set_listing(
            root.clone(),
            NotesDirectoryListing {
                entries: vec![
                    listing_entry(&root, "Inbox", NotesDirectoryEntryKind::Directory),
                    listing_entry(&root, "root-note.md", NotesDirectoryEntryKind::MarkdownFile),
                ],
                truncated: false,
            },
        );
        let m = test_module(
            Some(root.clone()),
            Arc::new(FakeOpenPath::new()),
            Arc::new(MemoryNotesIndex::new()),
            workspace,
        );
        let (tx, mut rx) = mpsc::channel(4);
        let q = Query::parse(format!("n browse {}", root.display()), 20);
        m.search(q, tx, CancellationToken::new()).await;
        let ev = rx.recv().await.expect("chunk");
        let Event::ResultsChunk { upserts, .. } = ev else {
            panic!("expected chunk");
        };
        assert!(upserts.iter().any(|u| u.title == "Inbox/"), "{upserts:?}");
        assert!(
            upserts.iter().any(|u| u.title == "root-note"),
            "{upserts:?}"
        );
        assert!(upserts.iter().all(|u| u.id != "notes:browse-denied"));
    }

    #[tokio::test]
    async fn browse_empty_folder_emits_status_row() {
        let root = PathBuf::from("/notes");
        let empty = root.join("Empty");
        let m = test_module(
            Some(root),
            Arc::new(FakeOpenPath::new()),
            Arc::new(MemoryNotesIndex::new()),
            Arc::new(FakeNotesWorkspace::new()),
        );
        let (tx, mut rx) = mpsc::channel(4);
        let q = Query::parse(format!("n browse {}", empty.display()), 20);
        m.search(q, tx, CancellationToken::new()).await;
        let ev = rx.recv().await.expect("chunk");
        let Event::ResultsChunk { upserts, .. } = ev else {
            panic!("expected chunk");
        };
        assert_eq!(upserts.len(), 1);
        assert_eq!(upserts[0].id, "notes:browse-empty");
        assert_eq!(upserts[0].kind, "status");
    }

    #[tokio::test]
    async fn search_no_matches_emits_status_row() {
        let m = test_module(
            Some(PathBuf::from("/notes")),
            Arc::new(FakeOpenPath::new()),
            Arc::new(MemoryNotesIndex::new()),
            Arc::new(FakeNotesWorkspace::new()),
        );
        let (tx, mut rx) = mpsc::channel(4);
        m.search(
            Query::parse("n zzz-not-a-real-note-xyz", 20),
            tx,
            CancellationToken::new(),
        )
        .await;
        let ev = rx.recv().await.expect("chunk");
        let Event::ResultsChunk { upserts, .. } = ev else {
            panic!("expected chunk");
        };
        assert!(
            upserts.iter().any(|u| u.id == "notes:no-matches"),
            "{upserts:?}"
        );
    }

    #[tokio::test]
    async fn browse_only_renders_vetted_adapter_entries() {
        let root = PathBuf::from("/notes");
        let workspace = Arc::new(FakeNotesWorkspace::new());
        workspace.set_listing(
            root.clone(),
            NotesDirectoryListing {
                entries: vec![listing_entry(
                    &root,
                    "safe.md",
                    NotesDirectoryEntryKind::MarkdownFile,
                )],
                truncated: false,
            },
        );
        let m = test_module(
            Some(root),
            Arc::new(FakeOpenPath::new()),
            Arc::new(MemoryNotesIndex::new()),
            workspace,
        );
        let (tx, mut rx) = mpsc::channel(4);
        m.search(Query::parse("n browse", 20), tx, CancellationToken::new())
            .await;
        let ev = rx.recv().await.expect("chunk");
        let Event::ResultsChunk { upserts, .. } = ev else {
            panic!("expected chunk");
        };
        assert!(
            upserts.iter().all(|u| !u.title.contains("unvetted")),
            "module must render only adapter-provided entries: {upserts:?}"
        );
        assert!(upserts.iter().any(|u| u.title == "safe"));
    }

    #[tokio::test]
    async fn preview_directory_lists_children() {
        let root = PathBuf::from("/notes");
        let workspace = Arc::new(FakeNotesWorkspace::new());
        workspace.set_preview(
            root.clone(),
            NotesWorkspacePreview::Directory(NotesDirectoryListing {
                entries: vec![
                    listing_entry(&root, "Backend", NotesDirectoryEntryKind::Directory),
                    listing_entry(&root, "readme.md", NotesDirectoryEntryKind::MarkdownFile),
                ],
                truncated: false,
            }),
        );
        let m = test_module(
            Some(root.clone()),
            Arc::new(FakeOpenPath::new()),
            Arc::new(MemoryNotesIndex::new()),
            workspace,
        );
        let item = SearchItem {
            id: ResultId::new(format!("browse:n:{}", root.display())),
            module_id: ModuleId::new("luma.notes"),
            title: "Learning/".into(),
            subtitle: Some(root.display().to_string()),
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
        };
        let preview = m.preview(&item).await.expect("directory preview");
        assert!(preview.contains("Backend/"));
        assert!(preview.contains("readme.md"));
        assert!(
            !preview.contains(&root.display().to_string()),
            "directory preview should list children, not repeat the path: {preview}"
        );
    }

    #[tokio::test]
    async fn preview_hides_adapter_rejected_item() {
        let root = PathBuf::from("/notes");
        let rejected = root.join("rejected.md");
        let workspace = Arc::new(FakeNotesWorkspace::new());
        workspace.fail_path(rejected.clone(), NotesWorkspaceError::InvalidItem);
        let m = test_module(
            Some(root.clone()),
            Arc::new(FakeOpenPath::new()),
            Arc::new(MemoryNotesIndex::new()),
            workspace,
        );
        let item = SearchItem {
            id: ResultId::new(format!("note:{}", rejected.display())),
            module_id: ModuleId::new("luma.notes"),
            title: "rejected".into(),
            subtitle: Some(rejected.display().to_string()),
            kind: "note".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("open"),
                label: "Open".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: None,
        };
        let preview = m.preview(&item).await;
        assert!(
            preview.is_none(),
            "rejected item must not have a preview: {preview:?}"
        );
    }

    #[tokio::test]
    async fn perform_browse_is_not_success() {
        let root = PathBuf::from("/notes");
        let m = test_module(
            Some(root.clone()),
            Arc::new(FakeOpenPath::new()),
            Arc::new(MemoryNotesIndex::new()),
            Arc::new(FakeNotesWorkspace::new()),
        );
        let out = m
            .perform(
                ActionRequest {
                    result: SearchItem {
                        id: ResultId::new(format!("browse:n:{}", root.display())),
                        module_id: ModuleId::new("luma.notes"),
                        title: "Browse".into(),
                        subtitle: Some(root.display().to_string()),
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
        assert!(matches!(out, ActionOutcome::Failed { .. }), "{out:?}");
    }

    #[tokio::test]
    async fn watcher_early_exit_surfaces_warning() {
        use luma_application::FakeMarkdownWatcher;
        use luma_domain::Query;
        let root = PathBuf::from("/notes");
        let watcher = Arc::new(FakeMarkdownWatcher {
            exit_immediately: true,
            ..Default::default()
        });
        let m = NotesModule::with_root_watcher_for_tests(Some(root), watcher);
        m.start_watch(CancellationToken::new()).await;
        // Auto-restart once, then surface warning on the second exit.
        tokio::time::sleep(Duration::from_millis(200)).await;
        let (tx, mut rx) = mpsc::channel(8);
        m.search(Query::parse("n ", 10), tx, CancellationToken::new())
            .await;
        let mut saw_warning = false;
        while let Ok(ev) = rx.try_recv() {
            if let Event::ResultsChunk { upserts, .. } = ev {
                if upserts.iter().any(|row| row.id == "notes:watch-warning") {
                    saw_warning = true;
                }
            }
        }
        assert!(
            saw_warning,
            "expected notes:watch-warning row after watcher exit"
        );
    }

    #[tokio::test]
    async fn apply_settings_same_excludes_skips_background_rebuild() {
        let root = PathBuf::from("/notes");
        let m = NotesModule::with_root_for_tests(Some(root.clone()));
        let gen_before = *m.index_generation.read().await;
        let settings = luma_application::AppSettings {
            notes_root: Some(root.display().to_string()),
            notes_exclude_patterns: Vec::new(),
            ..Default::default()
        };
        m.apply_settings(&settings).await;
        // Unchanged excludes must not spawn a rebuild (would race warmup and bump gen).
        tokio::time::sleep(Duration::from_millis(80)).await;
        assert_eq!(*m.index_generation.read().await, gen_before);
        assert!(m.watch_warning.lock().await.is_none());
    }

    #[tokio::test]
    async fn apply_settings_exclude_change_rebuilds_in_background() {
        let root = PathBuf::from("/notes");
        let m = NotesModule::with_root_for_tests(Some(root.clone()));
        let _ = m.rebuild_index(&root, None).await;
        let gen_before = *m.index_generation.read().await;
        let settings = luma_application::AppSettings {
            notes_root: Some(root.display().to_string()),
            notes_exclude_patterns: vec!["private/*".into()],
            ..Default::default()
        };
        m.apply_settings(&settings).await;
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(
            *m.index_generation.read().await > gen_before,
            "exclude change should background-rebuild and bump generation"
        );
    }
}
