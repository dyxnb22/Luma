use super::ProjectsModule;
use luma_application::{
    recipe_in_scope, GitError, GitProjectRoot, ImportedProject, ProjectWorkspaceError,
    RecallObject, RuntimeListener,
};
use luma_domain::{ActionRisk, VariantMatch};
use luma_protocol::{Event, SearchItemDto};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;

const MODULE_ID: &str = "luma.projects";
const RECALL_SCAN_LIMIT: usize = 1_000;

impl ProjectsModule {
    pub(super) fn project_editor(&self) -> Option<String> {
        let env = self.recipe_env.as_ref()?;
        ["code", "cursor", "zed", "nvim", "vim"]
            .into_iter()
            .find(|command| env.command_available(command))
            .map(str::to_string)
    }

    pub(super) async fn resolve_imported_project(
        &self,
        key: &str,
    ) -> Result<Option<ImportedProject>, String> {
        let imported = self.imported.read().await;
        if let Some(project) = imported.iter().find(|project| project.path == key) {
            return Ok(Some(project.clone()));
        }
        let key_lower = key.to_lowercase();
        let matches = imported
            .iter()
            .filter(|project| {
                project
                    .name
                    .as_deref()
                    .is_some_and(|name| name.to_lowercase() == key_lower)
                    || Path::new(&project.path)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.to_lowercase() == key_lower)
            })
            .cloned()
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [] => Ok(None),
            [project] => Ok(Some(project.clone())),
            _ => Err(format!(
                "multiple imported projects match `{key}`; use the full path"
            )),
        }
    }

    pub(super) fn project_name(project: &ImportedProject) -> String {
        project.name.clone().unwrap_or_else(|| {
            Path::new(&project.path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("project")
                .to_string()
        })
    }

    pub(super) fn project_activity_scores(
        &self,
        imported: &[ImportedProject],
    ) -> HashMap<String, f64> {
        let Some(recall) = &self.recall else {
            return HashMap::new();
        };
        let Ok(records) = recall.list_recent(RECALL_SCAN_LIMIT) else {
            return HashMap::new();
        };
        let mut scores = HashMap::<String, f64>::new();
        for (index, record) in records.into_iter().enumerate() {
            let Some(record_path) = record.project_path else {
                continue;
            };
            let Some(project) = imported
                .iter()
                .filter(|project| Path::new(&record_path).starts_with(Path::new(&project.path)))
                .max_by_key(|project| project.path.len())
            else {
                continue;
            };
            let recency = (30.0 - index.min(30) as f64).max(1.0);
            let frequency = ((record.use_count.max(1) as f64).log2() + 1.0).min(6.0) * 3.0;
            scores
                .entry(project.path.clone())
                .and_modify(|score| *score = score.max(recency + frequency))
                .or_insert(recency + frequency);
        }
        scores
    }

    fn project_payload(project: &ImportedProject) -> serde_json::Value {
        serde_json::json!({
            "project_path": project.path,
            "path": project.path,
            "name": Self::project_name(project),
            "browse_trigger": "proj",
        })
    }

    fn surface_row(
        project: &ImportedProject,
        suffix: &str,
        title: String,
        subtitle: String,
        score: f64,
        action: &str,
        label: &str,
    ) -> SearchItemDto {
        SearchItemDto {
            id: format!("proj:{suffix}:{}", project.path),
            module_id: MODULE_ID.into(),
            title,
            subtitle: Some(subtitle),
            kind: "project_surface".into(),
            score,
            primary_action_id: action.into(),
            primary_action_label: label.into(),
            action_payload: Some(Self::project_payload(project)),
            ..Default::default()
        }
    }

    fn recall_surface(record: &RecallObject, project_path: &str) -> Option<String> {
        match record.module_id.as_str() {
            "luma.git" => Some(format!(
                "/git repo {}",
                record.project_path.as_deref().unwrap_or(project_path)
            )),
            "luma.runtime" => Some(format!("/run {project_path}")),
            "luma.command_recipes" => Some(format!("/cmd project {project_path}")),
            MODULE_ID => match record.primary_action.as_str() {
                "open_git" => Some(format!("/git repo {project_path}")),
                "open_runtime" => Some(format!("/run {project_path}")),
                "open_recipes" => Some(format!("/cmd project {project_path}")),
                "open_files" => Some(format!("/proj browse {project_path}")),
                _ => None,
            },
            _ => None,
        }
    }

    fn latest_project_activity(&self, project_path: &str) -> Option<RecallObject> {
        self.recall
            .as_ref()?
            .list_recent(RECALL_SCAN_LIMIT)
            .ok()?
            .into_iter()
            .find(|record| {
                record
                    .project_path
                    .as_deref()
                    .is_some_and(|path| Path::new(path).starts_with(Path::new(project_path)))
                    && Self::recall_surface(record, project_path).is_some()
            })
    }

    pub(super) fn continue_surface(&self, project_path: &str) -> Option<String> {
        let record = self.latest_project_activity(project_path)?;
        Self::recall_surface(&record, project_path)
    }

    async fn runtime_rows_for_project(
        &self,
        project: &ImportedProject,
    ) -> Result<Vec<RuntimeListener>, String> {
        let Some(runtime) = &self.runtime else {
            return Err("Runtime context unavailable".into());
        };
        runtime
            .list_tcp_listeners()
            .await
            .map(|listeners| {
                listeners
                    .into_iter()
                    .filter(|listener| {
                        listener
                            .cwd
                            .as_deref()
                            .is_some_and(|cwd| cwd.starts_with(Path::new(&project.path)))
                    })
                    .collect()
            })
            .map_err(|error| error.to_string())
    }

    fn recipe_summary(&self, project: &ImportedProject) -> Result<String, String> {
        let Some(recipes) = &self.recipes else {
            return Err("Command Recipes metadata unavailable".into());
        };
        let Some(env) = &self.recipe_env else {
            return Err("Recipe environment unavailable".into());
        };
        let catalog = recipes.load_catalog();
        if catalog.has_fatal_issues() {
            return Err(catalog
                .issues
                .first()
                .map(|issue| format!("{}: {}", issue.location, issue.message))
                .unwrap_or_else(|| "Command Recipes unavailable".into()));
        }
        let base = Path::new(&project.path);
        let available = catalog
            .recipes
            .iter()
            .filter(|recipe| {
                recipe.enabled
                    && recipe_in_scope(env.as_ref(), base, &recipe.scope)
                    && matches!(
                        env.match_variant(base, &recipe.variants),
                        VariantMatch::Matched(_)
                    )
            })
            .count();
        Ok(match available {
            0 => "no matching recipes for this project".into(),
            1 => "1 matching recipe".into(),
            count => format!("{count} matching recipes"),
        })
    }

    pub(super) async fn search_project_workbench(
        &self,
        key: &str,
        limit: usize,
        sink: &luma_application::SearchSink,
        cancel: &CancellationToken,
    ) {
        let project = match self.resolve_imported_project(key).await {
            Ok(Some(project)) => project,
            Ok(None) => {
                self.send_workbench_status(
                    sink,
                    "proj:show-not-found",
                    "Imported project not found",
                    format!("No imported project matches `{key}`"),
                    "not_configured",
                )
                .await;
                return;
            }
            Err(reason) => {
                self.send_workbench_status(
                    sink,
                    "proj:show-ambiguous",
                    "Project name is ambiguous",
                    reason,
                    "unavailable",
                )
                .await;
                return;
            }
        };
        let project_path = PathBuf::from(&project.path);
        let available = match self
            .workspace
            .imported_project_statuses(vec![project_path], cancel.clone())
            .await
        {
            Ok(statuses) => statuses.first().copied().unwrap_or(false),
            Err(ProjectWorkspaceError::Cancelled) => return,
            Err(error) => {
                self.send_workbench_status(
                    sink,
                    "proj:show-unavailable",
                    "Project workspace unavailable",
                    error.to_string(),
                    "unavailable",
                )
                .await;
                return;
            }
        };
        if cancel.is_cancelled() {
            return;
        }
        if !available {
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![SearchItemDto {
                        id: format!("proj:missing:{}", project.path),
                        module_id: MODULE_ID.into(),
                        title: format!("{} is unavailable", Self::project_name(&project)),
                        subtitle: Some(format!("{} · path missing", project.path)),
                        kind: "unavailable".into(),
                        score: 100.0,
                        primary_action_id: "remove_project".into(),
                        primary_action_label: "Remove missing project".into(),
                        primary_action_risk: ActionRisk::Confirm,
                        primary_action_confirmation: true,
                        action_payload: Some(Self::project_payload(&project)),
                        ..Default::default()
                    }],
                    removed_ids: vec![],
                })
                .await;
            return;
        }

        let mut rows = vec![SearchItemDto {
            id: format!("proj:folder:{}", project.path),
            module_id: MODULE_ID.into(),
            title: Self::project_name(&project),
            subtitle: Some(project.path.clone()),
            kind: "project_header".into(),
            score: 100.0,
            primary_action_id: "open".into(),
            primary_action_label: "Open folder".into(),
            action_payload: Some(Self::project_payload(&project)),
            ..Default::default()
        }];

        if let Some(activity) = self.latest_project_activity(&project.path) {
            rows.push(SearchItemDto {
                id: format!("proj:continue:{}", project.path),
                module_id: MODULE_ID.into(),
                title: format!("Continue · {}", activity.title),
                subtitle: Some(format!(
                    "{} · used {} time{}",
                    activity.kind,
                    activity.use_count,
                    if activity.use_count == 1 { "" } else { "s" }
                )),
                kind: "project_continue".into(),
                score: 99.0,
                primary_action_id: "continue_project".into(),
                primary_action_label: "Continue".into(),
                action_payload: Some(Self::project_payload(&project)),
                ..Default::default()
            });
        }

        let git_subtitle = if let Some(git) = &self.git {
            match git
                .inspect(GitProjectRoot {
                    project_name: Self::project_name(&project),
                    path: PathBuf::from(&project.path),
                })
                .await
            {
                Ok(repo) => {
                    let mut parts = vec![
                        repo.branch.as_deref().unwrap_or("detached").to_string(),
                        if repo.is_dirty() {
                            format!(
                                "{} changed · {} conflict",
                                repo.files.len(),
                                repo.conflicted_count()
                            )
                        } else {
                            "clean".into()
                        },
                    ];
                    if repo.ahead > 0 || repo.behind > 0 {
                        parts.push(format!("↑{} ↓{}", repo.ahead, repo.behind));
                    }
                    parts.join(" · ")
                }
                Err(GitError::NotRepository(_)) => "not a Git repository".into(),
                Err(error) => error.to_string(),
            }
        } else {
            "Git context unavailable".into()
        };
        rows.push(Self::surface_row(
            &project,
            "git",
            "Git workbench".into(),
            git_subtitle,
            96.0,
            "open_git",
            "Open Git",
        ));

        let runtime_subtitle = match self.runtime_rows_for_project(&project).await {
            Ok(listeners) if listeners.is_empty() => "no associated TCP listeners".into(),
            Ok(listeners) => listeners
                .iter()
                .take(8)
                .map(|listener| format!("{} :{}", listener.process_name, listener.port))
                .collect::<Vec<_>>()
                .join(" · "),
            Err(reason) => reason,
        };
        rows.push(Self::surface_row(
            &project,
            "runtime",
            "Runtime listeners".into(),
            runtime_subtitle,
            95.0,
            "open_runtime",
            "Open Runtime",
        ));

        rows.push(Self::surface_row(
            &project,
            "recipes",
            "Command recipes".into(),
            self.recipe_summary(&project)
                .unwrap_or_else(|reason| reason),
            94.0,
            "open_recipes",
            "Open Recipes",
        ));
        rows.push(Self::surface_row(
            &project,
            "files",
            "Browse project files".into(),
            "bounded browsing inside configured project roots".into(),
            93.0,
            "open_files",
            "Browse files",
        ));
        if let Some(editor) = self.project_editor() {
            rows.push(SearchItemDto {
                id: format!("proj:editor:{}", project.path),
                module_id: MODULE_ID.into(),
                title: format!("Open in {editor}"),
                subtitle: Some(
                    "uses the first available editor CLI: code, cursor, zed, nvim, vim".into(),
                ),
                kind: "project_editor".into(),
                score: 92.0,
                primary_action_id: "open_editor".into(),
                primary_action_label: "Open editor".into(),
                action_payload: Some(Self::project_payload(&project)),
                ..Default::default()
            });
        }
        rows.push(SearchItemDto {
            id: format!("proj:terminal:{}", project.path),
            module_id: MODULE_ID.into(),
            title: "Open terminal here".into(),
            subtitle: Some("suspends Luma and starts an interactive zsh in the project".into()),
            kind: "project_terminal".into(),
            score: 91.0,
            primary_action_id: "open_terminal".into(),
            primary_action_label: "Open terminal".into(),
            action_payload: Some(Self::project_payload(&project)),
            ..Default::default()
        });
        rows.truncate(limit);
        if cancel.is_cancelled() {
            return;
        }
        let _ = sink
            .send(Event::ResultsChunk {
                request_id: String::new(),
                sequence: 1,
                upserts: rows,
                removed_ids: vec![],
            })
            .await;
    }

    async fn send_workbench_status(
        &self,
        sink: &luma_application::SearchSink,
        id: &str,
        title: &str,
        subtitle: String,
        kind: &str,
    ) {
        let _ = sink
            .send(Event::ResultsChunk {
                request_id: String::new(),
                sequence: 1,
                upserts: vec![SearchItemDto {
                    id: id.into(),
                    module_id: MODULE_ID.into(),
                    title: title.into(),
                    subtitle: Some(subtitle),
                    kind: kind.into(),
                    primary_action_id: "noop".into(),
                    primary_action_label: "Unavailable".into(),
                    ..Default::default()
                }],
                removed_ids: vec![],
            })
            .await;
    }
}
