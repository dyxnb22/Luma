use crate::cancel::await_unless_cancelled;
use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, ClockPort, FixedClock, LumaModule, MarkdownWatchPort,
    ModuleManifest, ModuleState, OpenPathPort, SearchMode, SearchSink, SystemClock, WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{Event, SearchItemDto};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug)]
struct NoteEntry {
    id: String,
    title: String,
    path: PathBuf,
}

/// No-op watcher for tests: waits on cancel without emitting change events.
pub(crate) struct NullMarkdownWatcher;

#[async_trait]
impl MarkdownWatchPort for NullMarkdownWatcher {
    async fn watch(&self, _root: PathBuf, cancel: CancellationToken, _tx: mpsc::Sender<()>) {
        cancel.cancelled().await;
    }
}

pub struct NotesModule {
    manifest: ModuleManifest,
    root: Arc<RwLock<Option<PathBuf>>>,
    index: Arc<RwLock<Vec<NoteEntry>>>,
    /// Bumped on each rebuild so late watch callbacks can be ignored if needed.
    index_generation: Arc<RwLock<u64>>,
    watch_cancel: Mutex<Option<CancellationToken>>,
    watch_handle: Mutex<Option<JoinHandle<()>>>,
    opener: Arc<dyn OpenPathPort>,
    watcher: Arc<dyn MarkdownWatchPort>,
    clock: Arc<dyn ClockPort>,
}

impl NotesModule {
    /// Production constructor used by the composition root.
    pub fn with_root(
        root: Option<PathBuf>,
        opener: Arc<dyn OpenPathPort>,
        watcher: Arc<dyn MarkdownWatchPort>,
    ) -> Self {
        Self::with_root_clock(root, opener, watcher, Arc::new(SystemClock))
    }

    pub fn with_root_clock(
        root: Option<PathBuf>,
        opener: Arc<dyn OpenPathPort>,
        watcher: Arc<dyn MarkdownWatchPort>,
        clock: Arc<dyn ClockPort>,
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
                    suggested_query: Some("n ".into()),
                    empty_hint: Some("n · n browse · n daily · n new".into()),
                    supports_browse: true,
                },
            },
            root: Arc::new(RwLock::new(root)),
            index: Arc::new(RwLock::new(Vec::new())),
            index_generation: Arc::new(RwLock::new(0)),
            watch_cancel: Mutex::new(None),
            watch_handle: Mutex::new(None),
            opener,
            watcher,
            clock,
        }
    }

    /// Test-only: Fake opener that never opens paths but reports success.
    pub fn with_root_for_tests(root: Option<PathBuf>) -> Self {
        use luma_application::FakeOpenPath;
        Self::with_root_and_opener(root, Arc::new(FakeOpenPath::new()))
    }

    /// Test helper: real opener mock + no-op markdown watcher.
    pub fn with_root_and_opener(root: Option<PathBuf>, opener: Arc<dyn OpenPathPort>) -> Self {
        Self::with_root_clock(
            root,
            opener,
            Arc::new(NullMarkdownWatcher),
            Arc::new(FixedClock {
                ymd: "2026-07-13".into(),
            }),
        )
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
        let (tx, mut rx) = mpsc::channel(8);
        let watcher = self.watcher.clone();
        let handle = tokio::spawn(async move {
            let watch_fut = watcher.watch(root.clone(), token.clone(), tx);
            tokio::pin!(watch_fut);
            loop {
                tokio::select! {
                    _ = token.cancelled() => break,
                    _ = &mut watch_fut => break,
                    Some(()) = rx.recv() => {
                        let current = module_root.read().await.clone();
                        if current.as_ref() != Some(&root) {
                            continue;
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
                            continue;
                        }
                        *gen = gen.saturating_add(1);
                        *index.write().await = notes;
                    }
                }
            }
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
    /// Rejects the notes root itself (create/write must not overwrite the workspace root).
    pub(crate) fn resolve_under_root(root: &Path, candidate: &Path) -> Result<PathBuf, String> {
        let resolved = Self::resolve_under_root_inner(root, candidate)?;
        let root_canon = root
            .canonicalize()
            .map_err(|e| format!("notes root not accessible: {e}"))?;
        if resolved == root_canon {
            return Err("refusing to overwrite notes root itself".into());
        }
        Ok(resolved)
    }

    /// Like [`resolve_under_root`], but allows `candidate` to resolve to the notes root
    /// itself (needed for `n browse` / `n browse <root>`).
    pub(crate) fn resolve_under_root_for_browse(
        root: &Path,
        candidate: &Path,
    ) -> Result<PathBuf, String> {
        Self::resolve_under_root_inner(root, candidate)
    }

    fn resolve_under_root_inner(root: &Path, candidate: &Path) -> Result<PathBuf, String> {
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
        if !resolved.starts_with(&root_canon) {
            return Err("path escapes notes root".into());
        }
        Ok(resolved)
    }

    /// Create parents and the new file via descriptor-relative `openat` / `mkdirat`
    /// with `O_NOFOLLOW` on every step so a swapped parent symlink cannot be followed.
    pub(crate) fn create_new_contained(
        root: &Path,
        candidate: &Path,
    ) -> Result<(PathBuf, std::fs::File), String> {
        let path = Self::resolve_under_root(root, candidate)?;
        let root_canon = root
            .canonicalize()
            .map_err(|e| format!("root canonicalize: {e}"))?;
        let rel = path
            .strip_prefix(&root_canon)
            .map_err(|_| "path not under notes root".to_string())?;
        let comps: Vec<_> = rel
            .components()
            .filter_map(|c| match c {
                Component::Normal(s) => Some(s.to_os_string()),
                _ => None,
            })
            .collect();
        if comps.is_empty() {
            return Err("refusing to overwrite notes root itself".into());
        }

        #[cfg(unix)]
        {
            use std::ffi::CString;
            use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
            use std::os::unix::ffi::OsStrExt;

            #[cfg(target_os = "macos")]
            const O_DIRECTORY: i32 = 0x0010_0000;
            #[cfg(target_os = "macos")]
            const O_NOFOLLOW: i32 = 0x0100;
            #[cfg(target_os = "macos")]
            const O_CREAT: i32 = 0x0200;
            #[cfg(target_os = "macos")]
            const O_EXCL: i32 = 0x0800;
            #[cfg(all(unix, not(target_os = "macos")))]
            const O_DIRECTORY: i32 = 0o200000;
            #[cfg(all(unix, not(target_os = "macos")))]
            const O_NOFOLLOW: i32 = 0o400000;
            #[cfg(all(unix, not(target_os = "macos")))]
            const O_CREAT: i32 = 0o100;
            #[cfg(all(unix, not(target_os = "macos")))]
            const O_EXCL: i32 = 0o200;
            const O_RDONLY: i32 = 0;
            const O_WRONLY: i32 = 1;

            extern "C" {
                fn open(path: *const i8, oflag: i32, ...) -> i32;
                fn openat(dirfd: i32, path: *const i8, oflag: i32, ...) -> i32;
                fn mkdirat(dirfd: i32, path: *const i8, mode: u32) -> i32;
            }

            let root_c = CString::new(root_canon.as_os_str().as_bytes())
                .map_err(|_| "root path contains NUL".to_string())?;
            let root_fd = unsafe { open(root_c.as_ptr(), O_RDONLY | O_DIRECTORY | O_NOFOLLOW) };
            if root_fd < 0 {
                return Err(format!("open root: {}", std::io::Error::last_os_error()));
            }
            let mut dir_fd = unsafe { OwnedFd::from_raw_fd(root_fd) };

            for name in &comps[..comps.len() - 1] {
                let cname = CString::new(name.as_bytes())
                    .map_err(|_| "path component contains NUL".to_string())?;
                let mut child = unsafe {
                    openat(
                        dir_fd.as_raw_fd(),
                        cname.as_ptr(),
                        O_RDONLY | O_DIRECTORY | O_NOFOLLOW,
                    )
                };
                if child < 0 {
                    let err = std::io::Error::last_os_error();
                    if err.kind() == std::io::ErrorKind::NotFound {
                        let mk = unsafe { mkdirat(dir_fd.as_raw_fd(), cname.as_ptr(), 0o755) };
                        if mk != 0 {
                            return Err(format!("mkdirat: {}", std::io::Error::last_os_error()));
                        }
                        child = unsafe {
                            openat(
                                dir_fd.as_raw_fd(),
                                cname.as_ptr(),
                                O_RDONLY | O_DIRECTORY | O_NOFOLLOW,
                            )
                        };
                        if child < 0 {
                            return Err(format!(
                                "openat after mkdir: {}",
                                std::io::Error::last_os_error()
                            ));
                        }
                    } else {
                        return Err(format!("openat parent: {err}"));
                    }
                }
                dir_fd = unsafe { OwnedFd::from_raw_fd(child) };
            }

            let file_name = &comps[comps.len() - 1];
            let cfile = CString::new(file_name.as_bytes())
                .map_err(|_| "filename contains NUL".to_string())?;
            let file_fd = unsafe {
                openat(
                    dir_fd.as_raw_fd(),
                    cfile.as_ptr(),
                    O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW,
                    0o644,
                )
            };
            if file_fd < 0 {
                return Err(format!(
                    "openat create: {}",
                    std::io::Error::last_os_error()
                ));
            }
            let file = unsafe { std::fs::File::from_raw_fd(file_fd) };
            Ok((path, file))
        }

        #[cfg(not(unix))]
        {
            let _ = comps;
            Err("contained create requires Unix openat".into())
        }
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

        let rest = query.rest_normalized();

        if rest == "new" {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let path = root.join("Inbox").join(format!("note-{stamp}.md"));
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

        if rest == "daily" || rest == "today" {
            let date = match self.clock.today_ymd() {
                Ok(d) => d,
                Err(err) => {
                    let _ = sink
                        .send(Event::ResultsChunk {
                            request_id: String::new(),
                            sequence: 1,
                            upserts: vec![SearchItemDto {
                                id: "note:clock-error".into(),
                                module_id: "luma.notes".into(),
                                title: "Daily note unavailable".into(),
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
                    return;
                }
            };
            let path = root.join("Daily").join(format!("{date}.md"));
            let exists = path.exists();
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![SearchItemDto {
                        id: format!("note:{}", path.display()),
                        module_id: "luma.notes".into(),
                        title: if exists {
                            format!("Open daily note ({date})")
                        } else {
                            format!("Create daily note ({date})")
                        },
                        subtitle: Some(path.display().to_string()),
                        kind: if exists {
                            "note".into()
                        } else {
                            "create".into()
                        },
                        score: 100.0,
                        primary_action_id: if exists {
                            "open".into()
                        } else {
                            "create".into()
                        },
                        primary_action_label: if exists {
                            "Open".into()
                        } else {
                            "Create".into()
                        },
                        ..Default::default()
                    }],
                    removed_ids: vec![],
                })
                .await;
            return;
        }

        let rest_raw = query.rest_raw();
        let rest_check = rest_raw.trim().to_lowercase();
        if rest_check == "browse"
            || rest_check.starts_with("browse ")
            || rest_check.starts_with("ls ")
        {
            let path_arg = rest_raw
                .trim()
                .strip_prefix("browse")
                .or_else(|| rest_raw.trim().strip_prefix("Browse"))
                .or_else(|| rest_raw.trim().strip_prefix("ls"))
                .or_else(|| rest_raw.trim().strip_prefix("LS"))
                .unwrap_or("")
                .trim();
            let dir = if path_arg.is_empty() {
                root.clone()
            } else {
                let candidate = PathBuf::from(path_arg);
                match NotesModule::resolve_under_root_for_browse(&root, &candidate) {
                    Ok(p) => p,
                    Err(err) => {
                        let _ = sink
                            .send(Event::ResultsChunk {
                                request_id: String::new(),
                                sequence: 1,
                                upserts: vec![SearchItemDto {
                                    id: "notes:browse-denied".into(),
                                    module_id: "luma.notes".into(),
                                    title: "Path outside notes root".into(),
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
                        return;
                    }
                }
            };
            let Ok(rd) = std::fs::read_dir(&dir) else {
                return;
            };
            let mut upserts = Vec::new();
            let mut entries: Vec<_> = rd.flatten().collect();
            entries.sort_by_key(|e| e.file_name());
            for entry in entries {
                if cancel.is_cancelled() {
                    return;
                }
                let path = entry.path();
                let Ok(meta) = std::fs::symlink_metadata(&path) else {
                    continue;
                };
                // Never follow or enumerate symlinks during browse.
                if meta.file_type().is_symlink() {
                    continue;
                }
                let name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("?")
                    .to_string();
                if name.starts_with('.') {
                    continue;
                }
                if meta.file_type().is_dir() {
                    upserts.push(SearchItemDto {
                        id: format!("browse:n:{}", path.display()),
                        module_id: "luma.notes".into(),
                        title: format!("{name}/"),
                        subtitle: Some(path.display().to_string()),
                        kind: "directory".into(),
                        score: 85.0,
                        primary_action_id: "browse".into(),
                        primary_action_label: "Browse".into(),
                        ..Default::default()
                    });
                } else if meta.file_type().is_file() && name.ends_with(".md") {
                    upserts.push(SearchItemDto {
                        id: format!("note:{}", path.display()),
                        module_id: "luma.notes".into(),
                        title: name.trim_end_matches(".md").to_string(),
                        subtitle: Some(path.display().to_string()),
                        kind: "note".into(),
                        score: 75.0,
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
            return;
        }

        let needle = rest;
        let index = self.index.read().await.clone();
        if needle.is_empty() {
            let mut upserts = vec![
                SearchItemDto {
                    id: format!("browse:n:{}", root.display()),
                    module_id: "luma.notes".into(),
                    title: "Browse notes folders".into(),
                    subtitle: Some(root.display().to_string()),
                    kind: "directory".into(),
                    score: 95.0,
                    primary_action_id: "browse".into(),
                    primary_action_label: "Browse".into(),
                    ..Default::default()
                },
                SearchItemDto {
                    id: "notes:open".into(),
                    module_id: "luma.notes".into(),
                    title: "Notes workspace".into(),
                    subtitle: Some(root.display().to_string()),
                    kind: "open".into(),
                    score: 1.0,
                    primary_action_id: "open".into(),
                    primary_action_label: "Open".into(),
                    ..Default::default()
                },
            ];
            for name in ["Inbox", "Daily"] {
                let path = root.join(name);
                if path.is_dir() {
                    upserts.push(SearchItemDto {
                        id: format!("browse:n:{}", path.display()),
                        module_id: "luma.notes".into(),
                        title: format!("{name}/"),
                        subtitle: Some(path.display().to_string()),
                        kind: "directory".into(),
                        score: 90.0,
                        primary_action_id: "browse".into(),
                        primary_action_label: "Browse".into(),
                        ..Default::default()
                    });
                }
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

    async fn actions(&self, result: &SearchItem) -> Vec<ActionDescriptor> {
        if result.id.as_str() == "notes:configure" {
            return vec![ActionDescriptor {
                id: ActionId::new("configure"),
                label: "Configure".into(),
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
        vec![ActionDescriptor {
            id: ActionId::new("open"),
            label: "Open".into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        }]
    }

    async fn preview(&self, result: &SearchItem) -> Option<String> {
        let root = self.root.read().await.clone()?;
        let raw = result.subtitle.as_deref().map(PathBuf::from)?;
        let path = Self::resolve_under_root(&root, &raw).ok()?;
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            return result.subtitle.clone();
        };
        if meta.file_type().is_symlink() || !meta.file_type().is_file() {
            return result.subtitle.clone();
        }
        let Ok(raw) = std::fs::read_to_string(&path) else {
            return result.subtitle.clone();
        };
        let mut body = raw.chars().take(4000).collect::<String>();
        if raw.chars().count() > 4000 {
            body.push_str("\n…");
        }
        Some(body)
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
            "browse" => ActionOutcome::Failed {
                kind: FailureKind::InvalidInput {
                    field: "action".into(),
                    message: "browse is search-driven; use `n browse <path>`".into(),
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
                self.rebuild_index(&root).await;
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
    use luma_application::FakeOpenPath;
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
        let q = Query::parse(&format!("n browse {}", dir.path().display()), 20);
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
}
