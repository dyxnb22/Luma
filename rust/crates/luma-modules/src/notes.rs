use crate::cancel::await_unless_cancelled;
use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, LumaModule, ModuleManifest, ModuleState, SearchMode, SearchSink,
    WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_platform_macos::{watch_markdown_root, FakeOpenPath, OpenPath};
use luma_protocol::{Event, SearchItemDto};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug)]
struct NoteEntry {
    id: String,
    title: String,
    path: PathBuf,
}

pub struct NotesModule {
    manifest: ModuleManifest,
    root: Arc<RwLock<Option<PathBuf>>>,
    index: Arc<RwLock<Vec<NoteEntry>>>,
    /// Bumped on each rebuild so late watch callbacks can be ignored if needed.
    index_generation: Arc<RwLock<u64>>,
    watch_cancel: Mutex<Option<CancellationToken>>,
    watch_handle: Mutex<Option<JoinHandle<()>>>,
    opener: Arc<dyn OpenPath>,
}

impl NotesModule {
    /// Production constructor used by the composition root.
    pub fn with_root(root: Option<PathBuf>, opener: Arc<dyn OpenPath>) -> Self {
        Self::with_root_and_opener(root, opener)
    }

    /// Test-only: Fake opener that never opens paths but reports success.
    pub fn with_root_for_tests(root: Option<PathBuf>) -> Self {
        Self::with_root_and_opener(root, Arc::new(FakeOpenPath::new()))
    }

    pub fn with_root_and_opener(root: Option<PathBuf>, opener: Arc<dyn OpenPath>) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.notes"),
                display_name: "Notes".into(),
                triggers: vec!["n".into(), "note".into(), "notes".into()],
                default_enabled: true,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
            },
            root: Arc::new(RwLock::new(root)),
            index: Arc::new(RwLock::new(Vec::new())),
            index_generation: Arc::new(RwLock::new(0)),
            watch_cancel: Mutex::new(None),
            watch_handle: Mutex::new(None),
            opener,
        }
    }

    pub async fn set_root_for_tests(&self, root: PathBuf) {
        *self.root.write().await = Some(root.clone());
        self.rebuild_index(&root).await;
    }

    async fn rebuild_index(&self, root: &Path) {
        let root = root.to_path_buf();
        let notes = tokio::task::spawn_blocking(move || scan_md_tree(&root))
            .await
            .unwrap_or_default();
        let mut gen = self.index_generation.write().await;
        *gen = gen.saturating_add(1);
        *self.index.write().await = notes;
    }

    async fn start_watch(&self, parent: CancellationToken) {
        self.stop_watch().await;
        let Some(root) = self.root.read().await.clone() else {
            return;
        };
        let cancel = parent.child_token();
        let token = cancel.clone();
        let module_root = self.root.clone();
        let index = self.index.clone();
        let generation = self.index_generation.clone();
        let handle = tokio::spawn(async move {
            watch_markdown_root(root.clone(), token, || {
                let module_root = module_root.clone();
                let index = index.clone();
                let generation = generation.clone();
                let root = root.clone();
                async move {
                    let current = module_root.read().await.clone();
                    if current.as_ref() != Some(&root) {
                        return;
                    }
                    let gen_before = *generation.read().await;
                    let notes = tokio::task::spawn_blocking({
                        let root = root.clone();
                        move || scan_md_tree(&root)
                    })
                    .await
                    .unwrap_or_default();
                    let mut gen = generation.write().await;
                    if *gen != gen_before {
                        return;
                    }
                    *gen = gen.saturating_add(1);
                    *index.write().await = notes;
                }
            })
            .await;
        });
        *self.watch_cancel.lock().await = Some(cancel);
        *self.watch_handle.lock().await = Some(handle);
    }

    async fn stop_watch(&self) {
        if let Some(cancel) = self.watch_cancel.lock().await.take() {
            cancel.cancel();
        }
        if let Some(handle) = self.watch_handle.lock().await.take() {
            let _ = handle.await;
        }
    }

    /// True when `candidate` exists and its canonical path is under canonical `root`.
    pub(crate) fn contained(root: &Path, candidate: &Path) -> bool {
        let Ok(root) = root.canonicalize() else {
            return false;
        };
        let Ok(cand) = candidate.canonicalize() else {
            return false;
        };
        cand.starts_with(&root)
    }

    /// Resolve a create/open target under `root` without allowing escape via `..`,
    /// absolute paths outside the root, or symlink redirection. Call **before** any write.
    pub(crate) fn resolve_under_root(root: &Path, candidate: &Path) -> Result<PathBuf, String> {
        let root_canon = root
            .canonicalize()
            .map_err(|e| format!("notes root not accessible: {e}"))?;

        let absolute = if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            for c in candidate.components() {
                match c {
                    Component::ParentDir => {
                        return Err("path escapes notes root (..)".into());
                    }
                    Component::RootDir | Component::Prefix(_) => {
                        return Err("absolute path segments not allowed in relative note id".into());
                    }
                    Component::CurDir | Component::Normal(_) => {}
                }
            }
            root_canon.join(candidate)
        };

        // Resolve through the longest existing ancestor so `/var` vs `/private/var`
        // and symlink roots compare correctly on macOS.
        let mut existing = absolute.clone();
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
        if missing.iter().any(|p| p == "..") {
            return Err("path escapes notes root (..)".into());
        }
        if !existing.exists() {
            return Err("path has no existing ancestor under notes root".into());
        }
        let mut resolved = existing
            .canonicalize()
            .map_err(|e| format!("cannot resolve path: {e}"))?;
        if !resolved.starts_with(&root_canon) {
            return Err("path escapes notes root".into());
        }
        for part in missing.into_iter().rev() {
            if part == ".." {
                return Err("path escapes notes root (..)".into());
            }
            if part == "." {
                continue;
            }
            resolved.push(part);
            if resolved.exists() {
                let canon = resolved
                    .canonicalize()
                    .map_err(|e| format!("cannot resolve path: {e}"))?;
                if !canon.starts_with(&root_canon) {
                    return Err("symlink escapes notes root".into());
                }
                resolved = canon;
            } else if !resolved.starts_with(&root_canon) {
                // Lexical push stayed under root_canon as Path prefix (both absolute).
                return Err("path escapes notes root".into());
            }
        }
        if resolved == root_canon {
            return Err("refusing to overwrite notes root itself".into());
        }
        if !resolved.starts_with(&root_canon) {
            return Err("path escapes notes root".into());
        }
        Ok(resolved)
    }

    /// Create parent directories only after containment is proven.
    pub(crate) fn create_with_containment(
        root: &Path,
        candidate: &Path,
    ) -> Result<PathBuf, String> {
        let path = Self::resolve_under_root(root, candidate)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("create_dir_all: {e}"))?;
            let parent_canon = parent
                .canonicalize()
                .map_err(|e| format!("parent canonicalize: {e}"))?;
            let root_canon = root
                .canonicalize()
                .map_err(|e| format!("root canonicalize: {e}"))?;
            if !parent_canon.starts_with(&root_canon) {
                return Err("parent escaped notes root after create".into());
            }
        }
        Ok(path)
    }
}

fn scan_md_tree(root: &Path) -> Vec<NoteEntry> {
    let mut notes = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    let root_canon = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            let Ok(meta) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            if meta.file_type().is_symlink() {
                // Never follow directory (or file) symlinks during indexing.
                continue;
            }
            if meta.is_dir() {
                if let Ok(canon) = path.canonicalize() {
                    if !canon.starts_with(&root_canon) {
                        continue;
                    }
                }
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) == Some("md") {
                let title = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("note")
                    .to_string();
                notes.push(NoteEntry {
                    id: format!("note:{}", path.display()),
                    title,
                    path,
                });
            }
            if notes.len() >= 5_000 {
                return notes;
            }
        }
    }
    notes
}

#[async_trait]
impl LumaModule for NotesModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, ctx: WarmupContext) -> ModuleState {
        let root = self.root.read().await.clone();
        if let Some(root) = root {
            if !ctx.cancel.is_cancelled() {
                self.rebuild_index(&root).await;
                self.start_watch(ctx.cancel).await;
            }
            ModuleState::Ready
        } else {
            ModuleState::Cold
        }
    }

    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        let root = self.root.read().await.clone();
        let Some(root) = root else {
            let row = SearchItemDto {
                id: "notes:configure".into(),
                module_id: "luma.notes".into(),
                title: "Choose a Notes root folder".into(),
                subtitle: Some("NotConfigured — set notes_root via luma config".into()),
                kind: "onboarding".into(),
                score: 0.0,
                primary_action_id: "configure".into(),
                primary_action_label: "Configure".into(),
                ..Default::default()
            };
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![row],
                    removed_ids: vec![],
                })
                .await;
            return;
        };

        let rest = query
            .normalized
            .split_once(|c: char| c.is_whitespace())
            .map(|(_, rest)| rest.trim().to_string())
            .unwrap_or_default();

        if rest == "new" {
            let path = root.join("Inbox").join(format!(
                "note-{}.md",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0)
            ));
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![SearchItemDto {
                        id: format!("note:{}", path.display()),
                        module_id: "luma.notes".into(),
                        title: "Create note".into(),
                        subtitle: Some(path.display().to_string()),
                        kind: "create".into(),
                        score: 100.0,
                        primary_action_id: "create".into(),
                        primary_action_label: "Create".into(),
                        ..Default::default()
                    }],
                    removed_ids: vec![],
                })
                .await;
            return;
        }

        let needle = rest;
        let index = self.index.read().await.clone();
        if needle.is_empty() {
            let row = SearchItemDto {
                id: "notes:open".into(),
                module_id: "luma.notes".into(),
                title: "Notes workspace".into(),
                subtitle: Some(root.display().to_string()),
                kind: "open".into(),
                score: 1.0,
                primary_action_id: "open".into(),
                primary_action_label: "Open".into(),
                ..Default::default()
            };
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![row],
                    removed_ids: vec![],
                })
                .await;
            return;
        }

        let mut upserts = Vec::new();
        for note in index {
            if cancel.is_cancelled() {
                return;
            }
            if NotesModule::resolve_under_root(&root, &note.path).is_err() {
                continue;
            }
            if note.title.to_lowercase().contains(&needle) {
                upserts.push(SearchItemDto {
                    id: note.id,
                    module_id: "luma.notes".into(),
                    title: note.title,
                    subtitle: Some(note.path.display().to_string()),
                    kind: "note".into(),
                    score: 70.0,
                    primary_action_id: "open".into(),
                    primary_action_label: "Open".into(),
                    ..Default::default()
                });
            }
            if upserts.len() >= query.limit {
                break;
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

    async fn actions(&self, _result: &SearchItem) -> Vec<ActionDescriptor> {
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
        let root = self.root.read().await.clone();
        match action.action.id.as_str() {
            "configure" => ActionOutcome::Failed {
                kind: FailureKind::NotConfigured {
                    remediation: "Set notes_root in LumaNext settings.toml".into(),
                },
            },
            "create" => {
                let Some(root) = root else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::NotConfigured {
                            remediation: "Set notes_root first".into(),
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
                let path = match Self::create_with_containment(&root, &raw) {
                    Ok(p) => p,
                    Err(reason) => {
                        return ActionOutcome::Failed {
                            kind: FailureKind::SecurityDenied { reason },
                        };
                    }
                };
                if let Err(err) = std::fs::write(&path, "# New note\n") {
                    return ActionOutcome::Failed {
                        kind: FailureKind::Io {
                            context: format!("write note: {err}"),
                        },
                    };
                }
                // Re-check containment after write (TOCTOU / symlink swap).
                if !Self::contained(&root, &path) {
                    let _ = std::fs::remove_file(&path);
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "created path escaped notes root".into(),
                        },
                    };
                }
                self.rebuild_index(&root).await;
                // Create success is independent of open. Best-effort open; failure is reported.
                match await_unless_cancelled(&cancel, self.opener.open(&path)).await {
                    None => ActionOutcome::Cancelled,
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
            other => ActionOutcome::Failed {
                kind: FailureKind::NotFound {
                    entity: format!("action:{other}"),
                },
            },
        }
    }

    async fn teardown(&self) {
        self.stop_watch().await;
        self.index.write().await.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ResultId, SearchItem};
    use std::fs;
    use std::os::unix::fs::symlink;
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
        m.search(Query::parse("n", 10), tx, CancellationToken::new())
            .await;
        let ev = rx.recv().await.unwrap();
        match ev {
            Event::ResultsChunk { upserts, .. } => {
                assert_eq!(upserts[0].kind, "onboarding");
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
        m.search(Query::parse("n hello", 10), tx, CancellationToken::new())
            .await;
        let ev = rx.recv().await.unwrap();
        match ev {
            Event::ResultsChunk { upserts, .. } => {
                assert_eq!(upserts[0].title, "hello");
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
        let notes = scan_md_tree(dir.path());
        assert!(
            notes.iter().any(|n| n.title == "safe"),
            "expected in-root note"
        );
        assert!(
            notes.iter().all(|n| n.title != "secret"),
            "directory symlink must not index outside root: {notes:?}"
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
}
