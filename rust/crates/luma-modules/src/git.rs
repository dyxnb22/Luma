//! Focused Git workbench: dashboard plus safe file-level working-tree actions.

use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, AppSettings, GitError, GitFile, GitProjectRoot,
    GitRepositoryPort, GitRepositoryState, ImportedProject, LumaModule, ModuleManifest,
    ModuleState, PasteboardPort, SearchMode, SearchSink, WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{Event, SearchItemDto, UiIntent};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

const MODULE_ID: &str = "luma.git";
const LOG_LIMIT: usize = 30;
pub struct GitModule {
    manifest: ModuleManifest,
    projects: Arc<RwLock<Vec<ImportedProject>>>,
    git: Arc<dyn GitRepositoryPort>,
    pasteboard: Arc<dyn PasteboardPort>,
    cached: Arc<RwLock<Vec<GitRepositoryState>>>,
}

impl GitModule {
    pub fn with_deps(
        projects: Vec<ImportedProject>,
        git: Arc<dyn GitRepositoryPort>,
        pasteboard: Arc<dyn PasteboardPort>,
    ) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new(MODULE_ID),
                display_name: "Git".into(),
                triggers: vec!["git".into()],
                default_enabled: true,
                search_mode: SearchMode::GlobalContributing,
                required_capabilities: vec![],
                workbench: luma_application::WorkbenchMeta {
                    glyph: Some("G".into()),
                    suggested_query: Some("/git ".into()),
                    empty_hint: Some("/git · /git repo PATH · /git commit MESSAGE".into()),
                    supports_browse: true,
                    commands: vec![
                        crate::ux::command_spec(
                            "/git [dirty|conflict|ahead|behind|clean|query]",
                            "Open or filter the imported-project Git dashboard",
                            "/git ",
                            Some("/git dirty"),
                        ),
                        crate::ux::command_spec(
                            "/git repo <imported-path>",
                            "Open a repository workbench",
                            "/git repo ",
                            Some("/git repo /Users/me/project"),
                        ),
                        crate::ux::command_spec(
                            "/git branches <imported-path>",
                            "List local branches for an imported repository",
                            "/git branches ",
                            Some("/git branches /Users/me/project"),
                        ),
                        crate::ux::command_spec(
                            "/git log <imported-path>",
                            "Show the bounded local commit log",
                            "/git log ",
                            Some("/git log /Users/me/project"),
                        ),
                        crate::ux::command_spec(
                            "/git commit <message>",
                            "Commit staged changes in the currently opened workbench",
                            "/git commit ",
                            Some("/git commit Fix parser"),
                        ),
                    ],
                },
            },
            projects: Arc::new(RwLock::new(projects)),
            git,
            pasteboard,
            cached: Arc::new(RwLock::new(Vec::new())),
        }
    }

    async fn project_roots(&self) -> Vec<GitProjectRoot> {
        self.projects
            .read()
            .await
            .iter()
            .map(|project| GitProjectRoot {
                project_name: project.name.clone().unwrap_or_else(|| {
                    PathBuf::from(&project.path)
                        .file_name()
                        .and_then(|value| value.to_str())
                        .unwrap_or("project")
                        .into()
                }),
                path: PathBuf::from(&project.path),
            })
            .collect()
    }

    async fn refresh_dashboard(&self) -> Vec<GitRepositoryState> {
        let repos = self.git.discover(self.project_roots().await).await;
        *self.cached.write().await = repos.clone();
        repos
    }

    async fn resolve_allowed_repo(&self, path: &str) -> Result<GitRepositoryState, GitError> {
        let requested = PathBuf::from(path);
        let Some(project) = self
            .project_roots()
            .await
            .into_iter()
            .find(|project| requested.starts_with(&project.path))
        else {
            return Err(GitError::Blocked(
                "repository is outside imported projects".into(),
            ));
        };
        let repo = self
            .git
            .inspect(GitProjectRoot {
                project_name: project.project_name,
                path: requested,
            })
            .await?;
        if !repo.path.starts_with(&project.path) {
            return Err(GitError::Blocked(
                "repository resolves outside the imported project".into(),
            ));
        }
        Ok(repo)
    }

    async fn cached_allowed_repo(&self, path: &std::path::Path) -> Option<GitRepositoryState> {
        let cached = self
            .cached
            .read()
            .await
            .iter()
            .find(|repo| repo.path == path)
            .cloned()?;
        self.project_roots()
            .await
            .iter()
            .any(|project| cached.path.starts_with(&project.path))
            .then_some(cached)
    }

    fn repo_payload(repo: &GitRepositoryState) -> serde_json::Value {
        serde_json::json!({
            "repo_path": repo.path.display().to_string(),
            "project_path": repo.path.display().to_string(),
            "project_name": repo.project_name,
            "branch": repo.branch,
            "staged_count": repo.staged_count(),
            "worktree_count": repo.unstaged_count()
                .saturating_add(repo.untracked_count())
                .saturating_add(repo.conflicted_count()),
            "surface_query": format!("/git repo {}", repo.path.display()),
        })
    }

    fn repo_row(repo: &GitRepositoryState, score: f64) -> SearchItemDto {
        let (kind, title, subtitle) = if let Some(reason) = &repo.unavailable {
            (
                "unavailable",
                format!("Git unavailable · {}", repo.project_name),
                reason.clone(),
            )
        } else {
            let state = if repo.conflicted_count() > 0 {
                "conflict"
            } else if repo.is_dirty() {
                "dirty"
            } else {
                "clean"
            };
            let branch = repo.branch.as_deref().unwrap_or("detached");
            let mut details = vec![
                format!("{state} · {branch}"),
                format!("staged {}", repo.staged_count()),
                format!("unstaged {}", repo.unstaged_count()),
                format!("untracked {}", repo.untracked_count()),
            ];
            if repo.ahead > 0 || repo.behind > 0 {
                details.push(format!("↑{} ↓{}", repo.ahead, repo.behind));
            }
            if let Some(commit) = &repo.last_commit {
                details.push(format!("{} {}", commit.short_sha, commit.subject));
            }
            ("git_repo", repo.project_name.clone(), details.join(" · "))
        };
        SearchItemDto {
            id: format!("git:repo:{}", repo.path.display()),
            module_id: MODULE_ID.into(),
            title,
            subtitle: Some(subtitle),
            kind: kind.into(),
            score,
            primary_action_id: if kind == "git_repo" {
                "open_workbench"
            } else {
                "noop"
            }
            .into(),
            primary_action_label: if kind == "git_repo" {
                "Open Git"
            } else {
                "Unavailable"
            }
            .into(),
            ui_intent: Some(UiIntent::OpenSurface).filter(|_| kind == "git_repo"),
            action_payload: Some(Self::repo_payload(repo)),
            ..Default::default()
        }
    }

    fn file_row(repo: &GitRepositoryState, file: &GitFile) -> SearchItemDto {
        let action = if file.staged { "unstage" } else { "stage" };
        let mut state = Vec::new();
        if file.conflicted {
            state.push("conflict");
        }
        if file.staged {
            state.push("staged");
        }
        if file.unstaged {
            state.push("unstaged");
        }
        if file.untracked {
            state.push("untracked");
        }
        SearchItemDto {
            id: format!("git:file:{}:{}", repo.path.display(), file.path),
            module_id: MODULE_ID.into(),
            title: file.path.clone(),
            subtitle: Some(state.join(" · ")),
            kind: "git_file".into(),
            score: 80.0,
            primary_action_id: action.into(),
            primary_action_label: if action == "stage" {
                "Stage"
            } else {
                "Unstage"
            }
            .into(),
            action_payload: Some(serde_json::json!({
                "repo_path": repo.path.display().to_string(),
                "project_path": repo.path.display().to_string(),
                "path": file.path,
                "staged": file.staged,
                "unstaged": file.unstaged,
                "untracked": file.untracked,
                "conflicted": file.conflicted,
                "repo_staged_count": repo.staged_count(),
                "repo_worktree_count": repo.unstaged_count()
                    .saturating_add(repo.untracked_count())
                    .saturating_add(repo.conflicted_count()),
            })),
            ..Default::default()
        }
    }

    async fn search_dashboard(
        &self,
        filter: &str,
        sink: &SearchSink,
        cancel: &CancellationToken,
        refresh: bool,
    ) {
        let repos = if refresh {
            self.refresh_dashboard().await
        } else {
            self.cached.read().await.clone()
        };
        if cancel.is_cancelled() {
            return;
        }
        let filter = filter.trim().to_lowercase();
        let mut rows = repos
            .into_iter()
            .filter(|repo| match filter.as_str() {
                "dirty" => repo.is_dirty() && repo.conflicted_count() == 0,
                "conflict" => repo.conflicted_count() > 0,
                "ahead" => repo.ahead > 0,
                "behind" => repo.behind > 0,
                "clean" => !repo.is_dirty() && repo.unavailable.is_none(),
                "" | "repos" => true,
                other => {
                    repo.project_name.to_lowercase().contains(other)
                        || repo
                            .path
                            .display()
                            .to_string()
                            .to_lowercase()
                            .contains(other)
                }
            })
            .collect::<Vec<_>>();
        rows.sort_by(|a, b| {
            (b.conflicted_count() > 0)
                .cmp(&(a.conflicted_count() > 0))
                .then_with(|| b.is_dirty().cmp(&a.is_dirty()))
                .then_with(|| {
                    b.ahead
                        .saturating_add(b.behind)
                        .cmp(&a.ahead.saturating_add(a.behind))
                })
                .then_with(|| {
                    a.project_name
                        .to_lowercase()
                        .cmp(&b.project_name.to_lowercase())
                })
        });
        let mut upserts = rows
            .iter()
            .enumerate()
            .map(|(index, repo)| Self::repo_row(repo, 100.0 - index as f64))
            .collect::<Vec<_>>();
        if upserts.is_empty() && !matches!(filter.as_str(), "") {
            return;
        }
        if upserts.is_empty() {
            upserts.push(SearchItemDto {
                id: "git:not-configured".into(),
                module_id: MODULE_ID.into(),
                title: "No imported Git repositories".into(),
                subtitle: Some("Import a project with /proj add PATH, then refresh /git".into()),
                kind: "not_configured".into(),
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

    async fn repo_by_path(&self, path: &str) -> Option<GitRepositoryState> {
        self.cached
            .read()
            .await
            .iter()
            .find(|repo| repo.path.display().to_string() == path)
            .cloned()
    }

    async fn search_workbench(&self, path: String, sink: &SearchSink, cancel: &CancellationToken) {
        let repo = match self.resolve_allowed_repo(&path).await {
            Ok(repo) => repo,
            Err(error) => {
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![unavailable_row(&path, error)],
                        removed_ids: vec![],
                    })
                    .await;
                return;
            }
        };
        if cancel.is_cancelled() {
            return;
        }
        *self.cached.write().await = vec![repo.clone()];
        let mut upserts = vec![Self::repo_row(&repo, 100.0)];
        if repo.unstaged_count() + repo.untracked_count() + repo.conflicted_count() > 0 {
            upserts.push(SearchItemDto {
                id: format!("git:stage-all:{}", repo.path.display()),
                module_id: MODULE_ID.into(),
                title: "Stage all tracked and untracked changes".into(),
                subtitle: Some(repo.path.display().to_string()),
                kind: "git_control".into(),
                score: 96.0,
                primary_action_id: "stage_all".into(),
                primary_action_label: "Stage all".into(),
                action_payload: Some(Self::repo_payload(&repo)),
                ..Default::default()
            });
        }
        if repo.staged_count() > 0 {
            upserts.push(SearchItemDto {
                id: format!("git:unstage-all:{}", repo.path.display()),
                module_id: MODULE_ID.into(),
                title: "Unstage all changes".into(),
                subtitle: Some(repo.path.display().to_string()),
                kind: "git_control".into(),
                score: 95.0,
                primary_action_id: "unstage_all".into(),
                primary_action_label: "Unstage all".into(),
                action_payload: Some(Self::repo_payload(&repo)),
                ..Default::default()
            });
        }
        upserts.push(SearchItemDto { id: format!("git:branches:{}", repo.path.display()), module_id: MODULE_ID.into(), title: "Branches".into(), subtitle: Some("switch is blocked when working tree is dirty".into()), kind: "git_surface".into(), score: 94.0, primary_action_id: "open_branches".into(), primary_action_label: "Branches".into(), ui_intent: Some(UiIntent::OpenSurface), action_payload: Some(serde_json::json!({ "surface_query": format!("/git branches {}", repo.path.display()), "repo_path": repo.path.display().to_string(), "project_path": repo.path.display().to_string() })), ..Default::default() });
        upserts.push(SearchItemDto { id: format!("git:log:{}", repo.path.display()), module_id: MODULE_ID.into(), title: "Recent commits".into(), subtitle: Some("local history only".into()), kind: "git_surface".into(), score: 93.0, primary_action_id: "open_log".into(), primary_action_label: "Log".into(), ui_intent: Some(UiIntent::OpenSurface), action_payload: Some(serde_json::json!({ "surface_query": format!("/git log {}", repo.path.display()), "repo_path": repo.path.display().to_string(), "project_path": repo.path.display().to_string() })), ..Default::default() });
        for file in &repo.files {
            upserts.push(Self::file_row(&repo, file));
        }
        upserts.truncate(cancelled_limit(cancel, 50));
        let _ = sink
            .send(Event::ResultsChunk {
                request_id: String::new(),
                sequence: 1,
                upserts,
                removed_ids: vec![],
            })
            .await;
    }

    async fn search_branches(&self, path: String, sink: &SearchSink) {
        let repo = match self.resolve_allowed_repo(&path).await {
            Ok(repo) => repo,
            Err(error) => {
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![unavailable_row(&path, error)],
                        removed_ids: vec![],
                    })
                    .await;
                return;
            }
        };
        let path = repo.path.display().to_string();
        *self.cached.write().await = vec![repo.clone()];
        match self.git.branches(repo.path).await {
            Ok(branches) => {
                let upserts = branches.into_iter().enumerate().map(|(index, branch)| SearchItemDto {
                    id: format!("git:branch:{path}:{}", branch.name), module_id: MODULE_ID.into(),
                    title: branch.name.clone(), subtitle: Some(if branch.current { "current branch".into() } else { "switch after a clean working tree".into() }),
                    kind: "git_branch".into(), score: 100.0 - index as f64, primary_action_id: "switch_branch".into(), primary_action_label: if branch.current { "Current" } else { "Switch" }.into(),
                    primary_action_risk: ActionRisk::Confirm, primary_action_confirmation: true,
                    action_payload: Some(serde_json::json!({ "repo_path": path, "project_path": path, "branch": branch.name })), ..Default::default()
                }).collect();
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts,
                        removed_ids: vec![],
                    })
                    .await;
            }
            Err(error) => {
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![unavailable_row(&path, error)],
                        removed_ids: vec![],
                    })
                    .await;
            }
        }
    }

    async fn search_log(&self, path: String, sink: &SearchSink) {
        let repo = match self.resolve_allowed_repo(&path).await {
            Ok(repo) => repo,
            Err(error) => {
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![unavailable_row(&path, error)],
                        removed_ids: vec![],
                    })
                    .await;
                return;
            }
        };
        let path = repo.path.display().to_string();
        *self.cached.write().await = vec![repo.clone()];
        match self.git.log(repo.path, LOG_LIMIT).await {
            Ok(commits) => {
                let upserts = commits.into_iter().enumerate().map(|(index, commit)| SearchItemDto { id: format!("git:commit-log:{path}:{}", commit.short_sha), module_id: MODULE_ID.into(), title: format!("{} {}", commit.short_sha, commit.subject), subtitle: Some(commit.authored_at), kind: "git_log".into(), score: 100.0 - index as f64, primary_action_id: "copy_sha".into(), primary_action_label: "Copy SHA".into(), action_payload: Some(serde_json::json!({ "repo_path": path, "project_path": path, "sha": commit.short_sha })), ..Default::default() }).collect();
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts,
                        removed_ids: vec![],
                    })
                    .await;
            }
            Err(error) => {
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![unavailable_row(&path, error)],
                        removed_ids: vec![],
                    })
                    .await;
            }
        }
    }

    fn repo_path(result: &SearchItem) -> Result<PathBuf, FailureKind> {
        result
            .action_payload
            .as_ref()
            .and_then(|payload| payload.get("repo_path"))
            .and_then(|value| value.as_str())
            .map(PathBuf::from)
            .ok_or_else(|| FailureKind::InvalidInput {
                field: "repository".into(),
                message: "missing Git repository path".into(),
            })
    }

    async fn copy(&self, value: String, label: &str) -> ActionOutcome {
        self.pasteboard
            .write_text(&value)
            .await
            .map(|_| ActionOutcome::Success {
                message: Some(format!("copied {label}")),
            })
            .unwrap_or_else(|error| ActionOutcome::Failed {
                kind: FailureKind::Unavailable {
                    reason: error.to_string(),
                    retryable: true,
                },
            })
    }
}

#[async_trait]
impl LumaModule for GitModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }
    async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
        self.refresh_dashboard().await;
        ModuleState::Ready
    }
    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        let rest = query.rest_raw().trim();
        if matches!(query.scope, luma_domain::QueryScope::Global) {
            self.search_dashboard(
                if query.normalized == "git" {
                    ""
                } else {
                    query.normalized.as_str()
                },
                &sink,
                &cancel,
                false,
            )
            .await;
            return;
        }
        if matches!(rest, "repo" | "branches" | "log") {
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![crate::ux::command_error(
                        MODULE_ID,
                        "git:path-command-invalid",
                        "Git command is incomplete",
                        format!("Usage: /git {rest} <imported-path>"),
                    )],
                    removed_ids: vec![],
                })
                .await;
            return;
        }
        if rest == "commit" {
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![crate::ux::command_error(
                        MODULE_ID,
                        "git:commit-invalid",
                        "Git commit command is incomplete",
                        "Usage: /git commit <message>",
                    )],
                    removed_ids: vec![],
                })
                .await;
            return;
        }
        if let Some(path) = rest
            .strip_prefix("repo ")
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            self.search_workbench(path.into(), &sink, &cancel).await;
            return;
        }
        if let Some(path) = rest
            .strip_prefix("branches ")
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            self.search_branches(path.into(), &sink).await;
            return;
        }
        if let Some(path) = rest
            .strip_prefix("log ")
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            self.search_log(path.into(), &sink).await;
            return;
        }
        if let Some(commit_spec) = rest.strip_prefix("commit ").map(str::trim) {
            // Commits always use the repository opened in the workbench. Accepting a path in
            // prompt text would turn a local surface into an arbitrary-filesystem command.
            let message = commit_spec;
            let repos = self.cached.read().await.clone();
            let repo = repos.first().cloned();
            if let Some(repo) = repo {
                let staged = repo.staged_count() > 0;
                let valid = staged && !message.is_empty();
                let _ = sink.send(Event::ResultsChunk { request_id: String::new(), sequence: 1, upserts: vec![SearchItemDto { id: format!("git:commit:{}", repo.path.display()), module_id: MODULE_ID.into(), title: if staged { "Commit staged changes".into() } else { "Nothing staged to commit".into() }, subtitle: Some(if !staged { "Stage one or more changes first".into() } else if message.is_empty() { "Type a non-empty commit message".into() } else { message.into() }), kind: if valid { "git_commit" } else { "status" }.into(), score: 100.0, primary_action_id: if valid { "commit" } else { "noop" }.into(), primary_action_label: if valid { "Commit" } else { "Unavailable" }.into(), primary_action_risk: if valid { ActionRisk::Confirm } else { ActionRisk::Safe }, primary_action_confirmation: valid, action_payload: Some(serde_json::json!({ "repo_path": repo.path.display().to_string(), "project_path": repo.path.display().to_string(), "message": message })), ..Default::default() }], removed_ids: vec![] }).await;
            } else {
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![crate::ux::command_error(
                            MODULE_ID,
                            "git:commit-no-workbench",
                            "No Git workbench is open",
                            "Open one with /git repo <imported-path>, then use /git commit <message>",
                        )],
                        removed_ids: vec![],
                    })
                    .await;
            }
            return;
        }
        self.search_dashboard(rest, &sink, &cancel, true).await;
    }
    async fn actions(&self, result: &SearchItem) -> Vec<ActionDescriptor> {
        match result.kind.as_str() {
            "git_file" => {
                let mut actions = vec![action(
                    if result.primary_action.id.as_str() == "stage" {
                        "stage"
                    } else {
                        "unstage"
                    },
                    result.primary_action.label.as_str(),
                    ActionRisk::Safe,
                )];
                let payload = result.action_payload.as_ref();
                if payload
                    .and_then(|value| value.get("repo_worktree_count"))
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0)
                    > 0
                {
                    actions.push(action("stage_all", "Stage all", ActionRisk::Safe));
                }
                if payload
                    .and_then(|value| value.get("repo_staged_count"))
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0)
                    > 0
                {
                    actions.push(action("unstage_all", "Unstage all", ActionRisk::Safe));
                }
                let unstaged = payload
                    .and_then(|value| value.get("unstaged"))
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false);
                let untracked = payload
                    .and_then(|value| value.get("untracked"))
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false);
                let conflicted = payload
                    .and_then(|value| value.get("conflicted"))
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false);
                if unstaged && !untracked && !conflicted {
                    actions.push(action(
                        "discard",
                        "Discard unstaged changes",
                        ActionRisk::Destructive,
                    ));
                }
                actions
            }
            "git_commit" => vec![action("commit", "Commit", ActionRisk::Confirm)],
            "git_control" | "git_log" => vec![action(
                result.primary_action.id.as_str(),
                result.primary_action.label.as_str(),
                ActionRisk::Safe,
            )],
            "git_surface" => vec![result.primary_action.clone()],
            "git_branch" => vec![action(
                "switch_branch",
                "Switch branch",
                ActionRisk::Confirm,
            )],
            "git_repo" => {
                let payload = result.action_payload.as_ref();
                let mut actions = Vec::new();
                if result.primary_action.id.as_str() == "open_workbench" {
                    actions.push(result.primary_action.clone());
                }
                if payload
                    .and_then(|value| value.get("worktree_count"))
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0)
                    > 0
                {
                    actions.push(action("stage_all", "Stage all", ActionRisk::Safe));
                }
                if payload
                    .and_then(|value| value.get("staged_count"))
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0)
                    > 0
                {
                    actions.push(action("unstage_all", "Unstage all", ActionRisk::Safe));
                }
                actions.push(action(
                    "copy_path",
                    "Copy repository path",
                    ActionRisk::Safe,
                ));
                if payload
                    .and_then(|value| value.get("branch"))
                    .is_some_and(|value| value.is_string())
                {
                    actions.push(action("copy_branch", "Copy branch", ActionRisk::Safe));
                }
                actions
            }
            _ => vec![action("noop", "OK", ActionRisk::Safe)],
        }
    }
    async fn preview(&self, result: &SearchItem) -> Option<String> {
        if result.kind == "git_file" {
            let repo = Self::repo_path(result).ok()?;
            let path = result
                .action_payload
                .as_ref()?
                .get("path")?
                .as_str()?
                .to_string();
            let staged = result
                .action_payload
                .as_ref()?
                .get("staged")
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            return self
                .git
                .diff(repo, path, staged)
                .await
                .ok()
                .map(|diff| diff.text);
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
        if action.action.confirmation && !action.confirmation {
            return ActionOutcome::Failed {
                kind: FailureKind::SecurityDenied {
                    reason: "confirmation required".into(),
                },
            };
        }
        let requested_repo = match Self::repo_path(&action.result) {
            Ok(repo) => repo,
            Err(kind) => return ActionOutcome::Failed { kind },
        };
        let repo = match self.cached_allowed_repo(&requested_repo).await {
            Some(repo) => repo.path,
            None => {
                return ActionOutcome::Failed {
                    kind: FailureKind::SecurityDenied {
                        reason: "repository is not an open imported project; refresh Git first"
                            .into(),
                    },
                }
            }
        };
        let path = action
            .result
            .action_payload
            .as_ref()
            .and_then(|payload| payload.get("path"))
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        let result = match action.action.id.as_str() {
            "open_workbench" | "open_branches" | "open_log" => {
                let Some(query) = action
                    .result
                    .action_payload
                    .as_ref()
                    .and_then(|payload| payload.get("surface_query"))
                    .and_then(|value| value.as_str())
                    .map(str::to_string)
                else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "surface_query".into(),
                            message: "missing Git surface route".into(),
                        },
                    };
                };
                return ActionOutcome::OpenSurface { query };
            }
            "stage" => self.git.stage(repo, path).await,
            "unstage" => self.git.unstage(repo, path).await,
            "stage_all" => self.git.stage_all(repo).await,
            "unstage_all" => self.git.unstage_all(repo).await,
            "discard" => self.git.discard(repo, path).await,
            "commit" => {
                let message = action
                    .result
                    .action_payload
                    .as_ref()
                    .and_then(|payload| payload.get("message"))
                    .and_then(|value| value.as_str())
                    .unwrap_or("")
                    .to_string();
                self.git.commit(repo, message).await
            }
            "switch_branch" => {
                let branch = action
                    .result
                    .action_payload
                    .as_ref()
                    .and_then(|payload| payload.get("branch"))
                    .and_then(|value| value.as_str())
                    .unwrap_or("")
                    .to_string();
                self.git.switch_branch(repo, branch).await
            }
            "copy_path" => {
                return self
                    .copy(repo.display().to_string(), "repository path")
                    .await
            }
            "copy_branch" => {
                let branch = self
                    .repo_by_path(&repo.display().to_string())
                    .await
                    .and_then(|state| state.branch);
                return self
                    .copy(branch.unwrap_or_else(|| "detached".into()), "branch")
                    .await;
            }
            "copy_sha" => {
                return self
                    .copy(
                        action
                            .result
                            .action_payload
                            .as_ref()
                            .and_then(|payload| payload.get("sha"))
                            .and_then(|value| value.as_str())
                            .unwrap_or("")
                            .into(),
                        "commit SHA",
                    )
                    .await
            }
            other => {
                return ActionOutcome::Failed {
                    kind: FailureKind::NotFound {
                        entity: format!("action:{other}"),
                    },
                }
            }
        };
        match result {
            Ok(()) => ActionOutcome::Success {
                message: Some("Git updated · press r or refresh".into()),
            },
            Err(error) => ActionOutcome::Failed {
                kind: git_failure(error),
            },
        }
    }
    async fn apply_settings(&self, settings: &AppSettings) {
        *self.projects.write().await = settings.imported_projects.clone();
        *self.cached.write().await = Vec::new();
    }
    async fn teardown(&self) {}
}

fn action(id: &str, label: &str, risk: ActionRisk) -> ActionDescriptor {
    ActionDescriptor {
        id: ActionId::new(id),
        label: label.into(),
        confirmation: !matches!(risk, ActionRisk::Safe),
        risk,
    }
}
fn cancelled_limit(cancel: &CancellationToken, max: usize) -> usize {
    if cancel.is_cancelled() {
        0
    } else {
        max
    }
}
fn unavailable_row(path: &str, error: GitError) -> SearchItemDto {
    SearchItemDto {
        id: format!("git:unavailable:{path}"),
        module_id: MODULE_ID.into(),
        title: "Git repository unavailable".into(),
        subtitle: Some(error.to_string()),
        kind: "unavailable".into(),
        primary_action_id: "noop".into(),
        primary_action_label: "Unavailable".into(),
        ..Default::default()
    }
}
fn git_failure(error: GitError) -> FailureKind {
    match error {
        GitError::Timeout => FailureKind::Timeout {
            operation: "git".into(),
        },
        GitError::Cancelled => FailureKind::Cancelled,
        GitError::InvalidInput(message) => FailureKind::InvalidInput {
            field: "git".into(),
            message,
        },
        GitError::NotRepository(entity) => FailureKind::NotFound { entity },
        GitError::Unavailable(reason) | GitError::Blocked(reason) => FailureKind::Unavailable {
            reason,
            retryable: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_application::{FakeGitRepository, FakePasteboard};
    use luma_test_support::collect_search_items;
    use tokio::sync::mpsc;

    fn query(raw: &str) -> Query {
        Query::parse_with_prefixes_strict(raw, 20, |prefix| prefix == "git")
    }

    fn repo() -> GitRepositoryState {
        GitRepositoryState {
            project_name: "Example".into(),
            path: PathBuf::from("/tmp/example"),
            branch: Some("main".into()),
            upstream: Some("origin/main".into()),
            ahead: 1,
            behind: 0,
            files: vec![GitFile {
                path: "src/main.rs".into(),
                previous_path: None,
                staged: false,
                unstaged: true,
                untracked: false,
                conflicted: false,
            }],
            last_commit: None,
            unavailable: None,
        }
    }
    #[tokio::test]
    async fn dashboard_prioritizes_dirty_repo_and_links_to_workbench() {
        let git = FakeGitRepository::new(vec![repo()]);
        let module = GitModule::with_deps(
            vec![ImportedProject {
                name: Some("Example".into()),
                path: "/tmp/example".into(),
            }],
            git.clone(),
            Arc::new(FakePasteboard::new()),
        );
        let (tx, mut rx) = mpsc::channel(4);
        module
            .search(query("/git "), tx, CancellationToken::new())
            .await;
        let Event::ResultsChunk { upserts, .. } = rx.recv().await.unwrap() else {
            panic!()
        };
        assert_eq!(upserts[0].kind, "git_repo");
        assert_eq!(upserts[0].ui_intent, Some(UiIntent::OpenSurface));
        assert_eq!(git.calls.lock().await.as_slice(), &["discover"]);
    }

    #[tokio::test]
    async fn global_search_uses_warmed_cache_without_rescanning_git() {
        let git = FakeGitRepository::new(vec![repo()]);
        let module = GitModule::with_deps(
            vec![ImportedProject {
                name: Some("Example".into()),
                path: "/tmp/example".into(),
            }],
            git.clone(),
            Arc::new(FakePasteboard::new()),
        );
        module
            .warmup(WarmupContext {
                cancel: CancellationToken::new(),
            })
            .await;

        let items = collect_search_items(&module, Query::parse("example", 20)).await;

        assert_eq!(items.len(), 1);
        assert_eq!(git.calls.lock().await.as_slice(), &["discover"]);
    }

    #[tokio::test]
    async fn incomplete_surface_commands_are_explicit_errors() {
        let git = FakeGitRepository::new(vec![repo()]);
        let module = GitModule::with_deps(
            vec![ImportedProject {
                name: Some("Example".into()),
                path: "/tmp/example".into(),
            }],
            git.clone(),
            Arc::new(FakePasteboard::new()),
        );
        for raw in ["/git repo", "/git branches", "/git log", "/git commit"] {
            let items = collect_search_items(&module, query(raw)).await;
            assert_eq!(items.len(), 1, "{raw}");
            assert_eq!(items[0].kind, "command_error", "{raw}");
        }
        let items = collect_search_items(&module, query("/git commit message")).await;
        assert_eq!(items[0].kind, "command_error");
        assert!(git.calls.lock().await.is_empty());
    }

    #[tokio::test]
    async fn direct_repo_path_outside_imported_projects_is_blocked() {
        let git = FakeGitRepository::new(vec![repo()]);
        let module = GitModule::with_deps(
            vec![ImportedProject {
                name: Some("Example".into()),
                path: "/tmp/example".into(),
            }],
            git,
            Arc::new(FakePasteboard::new()),
        );

        let items = collect_search_items(&module, query("/git repo /tmp/other")).await;

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, "unavailable");
        assert!(items[0]
            .subtitle
            .as_deref()
            .is_some_and(|subtitle| subtitle.contains("outside imported projects")));
    }

    #[tokio::test]
    async fn workbench_surface_rows_open_their_bound_queries() {
        let git = FakeGitRepository::new(vec![repo()]);
        let module = GitModule::with_deps(
            vec![ImportedProject {
                name: Some("Example".into()),
                path: "/tmp/example".into(),
            }],
            git,
            Arc::new(FakePasteboard::new()),
        );
        let items = collect_search_items(&module, query("/git repo /tmp/example")).await;
        let branches = items
            .into_iter()
            .find(|item| item.primary_action.id.as_str() == "open_branches")
            .unwrap();
        let outcome = module
            .perform(
                ActionRequest {
                    action: branches.primary_action.clone(),
                    result: branches,
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(
            outcome,
            ActionOutcome::OpenSurface { query }
                if query == "/git branches /tmp/example"
        ));
    }

    #[tokio::test]
    async fn discard_is_only_offered_for_safe_tracked_unstaged_rows() {
        let module = GitModule::with_deps(
            vec![],
            FakeGitRepository::new(vec![]),
            Arc::new(FakePasteboard::new()),
        );
        let tracked = GitModule::file_row(&repo(), &repo().files[0]).into_domain();
        let tracked_actions = module.actions(&tracked).await;
        assert!(tracked_actions
            .iter()
            .any(|action| action.id.as_str() == "discard"));

        let mut untracked_file = repo().files[0].clone();
        untracked_file.unstaged = false;
        untracked_file.untracked = true;
        let untracked = GitModule::file_row(&repo(), &untracked_file).into_domain();
        let untracked_actions = module.actions(&untracked).await;
        assert!(!untracked_actions
            .iter()
            .any(|action| action.id.as_str() == "discard"));
    }

    #[tokio::test]
    async fn clean_repository_hides_irrelevant_stage_controls() {
        let mut clean = repo();
        clean.files.clear();
        clean.ahead = 0;
        let git = FakeGitRepository::new(vec![clean.clone()]);
        let module = GitModule::with_deps(
            vec![ImportedProject {
                name: Some("Example".into()),
                path: "/tmp/example".into(),
            }],
            git,
            Arc::new(FakePasteboard::new()),
        );

        let actions = module
            .actions(&GitModule::repo_row(&clean, 100.0).into_domain())
            .await;
        assert_eq!(
            actions
                .iter()
                .map(|action| action.id.as_str())
                .collect::<Vec<_>>(),
            vec!["open_workbench", "copy_path", "copy_branch"]
        );

        let items = collect_search_items(&module, query("/git repo /tmp/example")).await;
        assert!(!items.iter().any(|item| {
            matches!(item.primary_action.id.as_str(), "stage_all" | "unstage_all")
        }));
    }
}
