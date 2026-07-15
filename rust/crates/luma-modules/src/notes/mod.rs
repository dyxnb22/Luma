use crate::cancel::await_unless_cancelled;
use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, ClockPort, FakePasteboard, FixedClock, LumaModule,
    MarkdownWatchPort, MemoryNotesIndex, ModuleManifest, ModuleState, NotesIndexRepository,
    OpenPathPort, PasteboardPort, SearchMode, SearchSink, SystemClock, WarmupContext,
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

mod path_security;
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
}

impl NotesModule {
    /// Production constructor used by the composition root.
    pub fn with_root(
        root: Option<PathBuf>,
        opener: Arc<dyn OpenPathPort>,
        watcher: Arc<dyn MarkdownWatchPort>,
        index: Arc<dyn NotesIndexRepository>,
        pasteboard: Arc<dyn PasteboardPort>,
        exclude_patterns: Vec<String>,
    ) -> Self {
        Self::with_root_clock(
            root,
            opener,
            watcher,
            index,
            pasteboard,
            Arc::new(SystemClock),
            exclude_patterns,
        )
    }

    pub fn with_root_clock(
        root: Option<PathBuf>,
        opener: Arc<dyn OpenPathPort>,
        watcher: Arc<dyn MarkdownWatchPort>,
        index: Arc<dyn NotesIndexRepository>,
        pasteboard: Arc<dyn PasteboardPort>,
        clock: Arc<dyn ClockPort>,
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
            clock,
        }
    }

    /// Test-only: Fake opener that never opens paths but reports success.
    pub fn with_root_for_tests(root: Option<PathBuf>) -> Self {
        use luma_application::FakeOpenPath;
        Self::with_root_and_opener(root, Arc::new(FakeOpenPath::new()))
    }

    /// Test helper: real opener mock + no-op markdown watcher + memory index.
    pub fn with_root_and_opener(root: Option<PathBuf>, opener: Arc<dyn OpenPathPort>) -> Self {
        Self::with_root_clock(
            root,
            opener,
            Arc::new(watch::NullMarkdownWatcher),
            Arc::new(MemoryNotesIndex::new()),
            Arc::new(FakePasteboard::new()),
            Arc::new(FixedClock {
                ymd: "2026-07-13".into(),
            }),
            Vec::new(),
        )
    }

    /// Test helper with a custom markdown watcher.
    pub fn with_root_watcher_for_tests(
        root: Option<PathBuf>,
        watcher: Arc<dyn MarkdownWatchPort>,
    ) -> Self {
        use luma_application::FakeOpenPath;
        Self::with_root_clock(
            root,
            Arc::new(FakeOpenPath::new()),
            watcher,
            Arc::new(MemoryNotesIndex::new()),
            Arc::new(FakePasteboard::new()),
            Arc::new(FixedClock {
                ymd: "2026-07-13".into(),
            }),
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
                let (path, mut file) = match Self::create_new_contained(&root, &raw) {
                    Ok(v) => v,
                    Err(reason) => {
                        return ActionOutcome::Failed {
                            kind: FailureKind::SecurityDenied { reason },
                        };
                    }
                };
                if let Err(err) = {
                    use std::io::Write;
                    let body = if path.to_string_lossy().contains("/Daily/") {
                        let date = path.file_stem().and_then(|s| s.to_str()).unwrap_or("today");
                        format!("# {date}\n\n")
                    } else {
                        "# New note\n".into()
                    };
                    file.write_all(body.as_bytes())
                } {
                    let _ = std::fs::remove_file(&path);
                    return ActionOutcome::Failed {
                        kind: FailureKind::Io {
                            context: format!("write note: {err}"),
                        },
                    };
                }
                // Re-check containment after write (defense in depth).
                if !Self::contained(&root, &path) {
                    let _ = std::fs::remove_file(&path);
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "created path escaped notes root".into(),
                        },
                    };
                }
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
                    // Opening workspace root directory.
                    return match await_unless_cancelled(&cancel, self.opener.open(&root)).await {
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
                let path = match Self::resolve_under_root(&root, &raw) {
                    Ok(p) => p,
                    Err(reason) => {
                        return ActionOutcome::Failed {
                            kind: FailureKind::SecurityDenied { reason },
                        };
                    }
                };
                if !path.exists() {
                    return ActionOutcome::Failed {
                        kind: FailureKind::NotFound {
                            entity: path.display().to_string(),
                        },
                    };
                }
                if !Self::contained(&root, &path) {
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "path escapes notes root".into(),
                        },
                    };
                }
                // Re-check immediately before open to shrink symlink-swap window.
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
                if !Self::contained(&root, &path) {
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "path escapes notes root".into(),
                        },
                    };
                }
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
                let path = match Self::resolve_under_root(&root, &raw) {
                    Ok(p) => p,
                    Err(reason) => {
                        return ActionOutcome::Failed {
                            kind: FailureKind::SecurityDenied { reason },
                        };
                    }
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
    use luma_application::{FakeOpenPath, MemoryNotesIndex};
    use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ResultId, SearchItem};
    use luma_protocol::Event;
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::path::Path;
    use std::time::Duration;
    use tempfile::tempdir;
    use tokio::sync::mpsc;

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
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("hello.md"), "# hi").unwrap();
        let m = NotesModule::with_root_for_tests(None);
        m.set_root_for_tests(dir.path().to_path_buf()).await;
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

    #[test]
    fn containment_helper_requires_canonical() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("safe.md"), "x").unwrap();
        assert!(NotesModule::contained(
            dir.path(),
            &dir.path().join("safe.md")
        ));
    }

    #[test]
    fn rejects_parent_dir_escape() {
        let dir = tempdir().unwrap();
        let outside = dir.path().join("escape.md");
        let err =
            NotesModule::resolve_under_root(dir.path(), Path::new("../escape.md")).unwrap_err();
        assert!(err.contains("escape") || err.contains(".."), "{err}");
        assert!(!outside.exists());
    }

    #[test]
    fn rejects_absolute_outside_root() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let target = outside.path().join("evil.md");
        let err = NotesModule::resolve_under_root(dir.path(), &target).unwrap_err();
        assert!(err.contains("escape"), "{err}");
    }

    #[test]
    fn rejects_symlink_escape() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        fs::write(outside.path().join("secret.md"), "nope").unwrap();
        let link = dir.path().join("link");
        symlink(outside.path(), &link).unwrap();
        let err = NotesModule::resolve_under_root(dir.path(), &link.join("secret.md")).unwrap_err();
        assert!(err.contains("escape") || err.contains("symlink"), "{err}");
    }

    #[test]
    fn scan_skips_directory_symlink_outside_root() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        fs::write(outside.path().join("secret.md"), "nope").unwrap();
        fs::write(dir.path().join("safe.md"), "ok").unwrap();
        let link = dir.path().join("escape-link");
        symlink(outside.path(), &link).unwrap();
        let index = MemoryNotesIndex::new();
        index.full_scan(dir.path(), None).unwrap();
        let hits = index.search("safe", 20).unwrap();
        assert!(!hits.is_empty(), "expected in-root note");
        let secret = index.search("secret", 20).unwrap();
        assert!(
            secret.is_empty(),
            "directory symlink must not index outside root: {secret:?}"
        );
    }

    #[tokio::test]
    async fn create_rejects_dotdot_before_write() {
        let dir = tempdir().unwrap();
        let outside = dir
            .path()
            .parent()
            .unwrap()
            .join(format!("luma-notes-escape-{}.md", std::process::id()));
        let fake = Arc::new(FakeOpenPath::new());
        let m = NotesModule::with_root_and_opener(Some(dir.path().to_path_buf()), fake);
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
        assert!(!outside.exists() || !outside.ends_with("escaped.md"));
        // Ensure nothing written beside root as ../escaped.md relative to root's parent.
        let sibling = dir.path().parent().unwrap().join("escaped.md");
        assert!(!sibling.exists(), "escaped file must not be created");
    }

    #[tokio::test]
    async fn create_success_even_if_open_fails() {
        let dir = tempdir().unwrap();
        let fake = Arc::new(FakeOpenPath::with_failure());
        let m = NotesModule::with_root_and_opener(Some(dir.path().to_path_buf()), fake.clone());
        let path = dir.path().join("Inbox").join("a.md");
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
        assert!(path.exists());
        assert_eq!(fake.open_count.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn open_failure_is_not_success() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.md"), "x").unwrap();
        let fake = Arc::new(FakeOpenPath::with_failure());
        let m = NotesModule::with_root_and_opener(Some(dir.path().to_path_buf()), fake);
        let out = m
            .perform(
                open_request(&dir.path().join("a.md")),
                CancellationToken::new(),
            )
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
        let dir = tempdir().unwrap();
        let fake = Arc::new(FakeOpenPath::new());
        let m = NotesModule::with_root_and_opener(Some(dir.path().to_path_buf()), fake);
        let path = PathBuf::from("Inbox/hello.md");
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
        assert!(dir.path().join("Inbox/hello.md").exists());
        let _ = path;
    }

    #[test]
    fn browse_resolve_allows_notes_root() {
        let dir = tempdir().unwrap();
        let resolved = NotesModule::resolve_under_root_for_browse(dir.path(), dir.path()).unwrap();
        assert_eq!(resolved, dir.path().canonicalize().unwrap());
        let err = NotesModule::resolve_under_root(dir.path(), dir.path()).unwrap_err();
        assert!(err.contains("overwrite") || err.contains("root"), "{err}");
    }

    #[tokio::test]
    async fn browse_root_lists_children() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("Inbox")).unwrap();
        fs::write(dir.path().join("root-note.md"), "# root").unwrap();
        let m = NotesModule::with_root_for_tests(Some(dir.path().to_path_buf()));
        let (tx, mut rx) = mpsc::channel(4);
        let q = Query::parse(format!("n browse {}", dir.path().display()), 20);
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
        let dir = tempdir().unwrap();
        let empty = dir.path().join("Empty");
        fs::create_dir(&empty).unwrap();
        let m = NotesModule::with_root_for_tests(Some(dir.path().to_path_buf()));
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
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("alpha.md"), "# alpha\nhello").unwrap();
        let m = NotesModule::with_root_for_tests(Some(dir.path().to_path_buf()));
        m.set_root_for_tests(dir.path().to_path_buf()).await;
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
    async fn browse_skips_symlink_escape() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        fs::write(outside.path().join("secret.md"), "LEAK").unwrap();
        symlink(
            outside.path().join("secret.md"),
            dir.path().join("secret.md"),
        )
        .unwrap();
        fs::write(dir.path().join("safe.md"), "ok").unwrap();
        let m = NotesModule::with_root_for_tests(Some(dir.path().to_path_buf()));
        let (tx, mut rx) = mpsc::channel(4);
        m.search(Query::parse("n browse", 20), tx, CancellationToken::new())
            .await;
        let ev = rx.recv().await.expect("chunk");
        let Event::ResultsChunk { upserts, .. } = ev else {
            panic!("expected chunk");
        };
        assert!(
            upserts.iter().all(|u| !u.title.contains("secret")),
            "symlink must not appear in browse: {upserts:?}"
        );
        assert!(upserts.iter().any(|u| u.title == "safe"));
    }

    #[tokio::test]
    async fn preview_directory_lists_children() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("Backend")).unwrap();
        fs::write(dir.path().join("readme.md"), "# hi").unwrap();
        let m = NotesModule::with_root_for_tests(Some(dir.path().to_path_buf()));
        let item = SearchItem {
            id: ResultId::new(format!("browse:n:{}", dir.path().display())),
            module_id: ModuleId::new("luma.notes"),
            title: "Learning/".into(),
            subtitle: Some(dir.path().display().to_string()),
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
            !preview.contains(dir.path().to_str().unwrap()),
            "directory preview should list children, not repeat the path: {preview}"
        );
    }

    #[tokio::test]
    async fn preview_does_not_leak_symlink_outside() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let secret = outside.path().join("secret.md");
        fs::write(&secret, "TOPSECRET").unwrap();
        let link = dir.path().join("secret.md");
        symlink(&secret, &link).unwrap();
        let m = NotesModule::with_root_for_tests(Some(dir.path().to_path_buf()));
        let item = SearchItem {
            id: ResultId::new(format!("note:{}", link.display())),
            module_id: ModuleId::new("luma.notes"),
            title: "secret".into(),
            subtitle: Some(link.display().to_string()),
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
            preview
                .as_deref()
                .map(|p| !p.contains("TOPSECRET"))
                .unwrap_or(true),
            "preview must not leak symlink target: {preview:?}"
        );
    }

    #[tokio::test]
    async fn perform_browse_is_not_success() {
        let dir = tempdir().unwrap();
        let m = NotesModule::with_root_for_tests(Some(dir.path().to_path_buf()));
        let out = m
            .perform(
                ActionRequest {
                    result: SearchItem {
                        id: ResultId::new(format!("browse:n:{}", dir.path().display())),
                        module_id: ModuleId::new("luma.notes"),
                        title: "Browse".into(),
                        subtitle: Some(dir.path().display().to_string()),
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
        let dir = tempdir().unwrap();
        let watcher = Arc::new(FakeMarkdownWatcher {
            exit_immediately: true,
            ..Default::default()
        });
        let m = NotesModule::with_root_watcher_for_tests(Some(dir.path().to_path_buf()), watcher);
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
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
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
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.md"), "hello").unwrap();
        let root = dir.path().to_path_buf();
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
