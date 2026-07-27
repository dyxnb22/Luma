use crate::cancel::await_unless_cancelled;
use async_trait::async_trait;
use luma_application::{
    recipe_in_scope, recipe_runnable, resolve_steps, ActionOutcome, ActionRequest, AppSettings,
    CommandRecipesRepository, ImportedProject, LumaModule, ModuleManifest, ModuleState,
    OpenPathPort, PasteboardPort, RecipeEnvironmentPort, SearchMode, SearchSink, WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, Recipe, RecipeMetadata,
    RecipeRisk, RecipeRunPlan, SearchItem, VariantMatch,
};
use luma_protocol::{Event, SearchItemDto};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

const MODULE_ID: &str = "luma.command_recipes";

struct RecipeQueryContextError {
    title: &'static str,
    kind: &'static str,
    reason: String,
}

pub struct CommandRecipesModule {
    manifest: ModuleManifest,
    repo: Arc<dyn CommandRecipesRepository>,
    env: Arc<dyn RecipeEnvironmentPort>,
    pasteboard: Arc<dyn PasteboardPort>,
    opener: Arc<dyn OpenPathPort>,
    projects: Arc<RwLock<Vec<ImportedProject>>>,
}

impl CommandRecipesModule {
    /// Canonical command discovery owned by this module, including unavailable fallbacks.
    pub fn command_specs() -> Vec<luma_application::CommandSpec> {
        vec![
            crate::ux::command_spec(
                "/cmd [filter]",
                "List runnable recipes in the current directory",
                "/cmd ",
                Some("/cmd test"),
            ),
            crate::ux::command_spec(
                "/cmd all [filter]",
                "Include inapplicable recipes after runnable rows",
                "/cmd all ",
                Some("/cmd all rust"),
            ),
            crate::ux::command_spec(
                "/cmd project <imported-path>",
                "Evaluate recipes against one exact imported project",
                "/cmd project ",
                Some("/cmd project /Users/me/project"),
            ),
        ]
    }

    pub fn with_deps(
        repo: Arc<dyn CommandRecipesRepository>,
        env: Arc<dyn RecipeEnvironmentPort>,
        pasteboard: Arc<dyn PasteboardPort>,
        opener: Arc<dyn OpenPathPort>,
    ) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new(MODULE_ID),
                display_name: "Command Recipes".into(),
                triggers: vec!["cmd".into(), "recipe".into(), "recipes".into()],
                default_enabled: true,
                search_mode: SearchMode::GlobalContributing,
                required_capabilities: vec![],
                workbench: luma_application::WorkbenchMeta {
                    glyph: Some("C".into()),
                    suggested_query: Some("/cmd ".into()),
                    empty_hint: Some("/cmd · /cmd test · r run · c copy · f favorite".into()),
                    supports_browse: false,
                    commands: Self::command_specs(),
                },
            },
            repo,
            env,
            pasteboard,
            opener,
            projects: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn with_projects(mut self, projects: Vec<ImportedProject>) -> Self {
        self.projects = Arc::new(RwLock::new(projects));
        self
    }

    fn refresh_catalog(&self) -> luma_domain::RecipeCatalog {
        self.repo.load_catalog()
    }

    fn recipe_id_from_result(result_id: &str) -> Option<String> {
        result_id.strip_prefix("cmd:").map(str::to_string)
    }

    fn risk_to_action(risk: &RecipeRisk) -> ActionRisk {
        match risk {
            RecipeRisk::Safe => ActionRisk::Safe,
            RecipeRisk::Confirm => ActionRisk::Confirm,
            RecipeRisk::Destructive => ActionRisk::Destructive,
        }
    }

    fn scope_visible(&self, recipe: &Recipe, base: &std::path::Path) -> bool {
        recipe_in_scope(self.env.as_ref(), base, &recipe.scope)
    }

    fn format_subtitle(
        recipe: &Recipe,
        meta: &RecipeMetadata,
        variant: Option<&str>,
        matched: bool,
    ) -> String {
        let mut parts = vec![format!("risk: {}", recipe.risk.as_str())];
        if let Some(v) = variant {
            parts.push(format!("variant: {v}"));
        }
        if !matched {
            parts.push("当前项目不适用".into());
        }
        if meta.favorite {
            parts.push("★".into());
        }
        if meta.use_count > 0 {
            parts.push(format!("used {}", meta.use_count));
        }
        if let Some(ts) = meta.last_used_at {
            parts.push(format!("last {ts}"));
        }
        if !recipe.tags.is_empty() {
            parts.push(recipe.tags.join(", "));
        }
        parts.join(" · ")
    }

    fn score_recipe(recipe: &Recipe, query: &str, meta: &RecipeMetadata) -> f64 {
        let q = query.trim().to_ascii_lowercase();
        if q.is_empty() {
            let base = 50.0;
            return base + if meta.favorite { 20.0 } else { 0.0 };
        }
        let id = recipe.id.to_ascii_lowercase();
        let title = recipe.title.to_ascii_lowercase();
        let mut score = 0.0;
        if id == q {
            score = 100.0;
        } else if id.starts_with(&q) {
            score = 90.0;
        } else if title.contains(&q) {
            score = 80.0;
        } else if recipe
            .tags
            .iter()
            .any(|t| t.to_ascii_lowercase().contains(&q))
        {
            score = 70.0;
        } else if recipe.description.to_ascii_lowercase().contains(&q) {
            score = 60.0;
        }
        if meta.favorite {
            score += 5.0;
        }
        score
    }

    #[allow(clippy::result_large_err)]
    fn build_plan_at(&self, recipe: &Recipe, base: &Path) -> Result<RecipeRunPlan, FailureKind> {
        recipe_runnable(self.env.as_ref(), base, recipe).map_err(|message| {
            FailureKind::InvalidInput {
                field: "recipe".into(),
                message,
            }
        })?;
        let variant = match self.env.match_variant(base, &recipe.variants) {
            VariantMatch::Matched(v) => v,
            VariantMatch::NoMatch => {
                return Err(FailureKind::InvalidInput {
                    field: "variant".into(),
                    message: "当前项目不适用".into(),
                });
            }
        };
        let steps = resolve_steps(self.env.as_ref(), base, &variant).map_err(|e| {
            FailureKind::InvalidInput {
                field: "cwd".into(),
                message: e.0,
            }
        })?;
        Ok(RecipeRunPlan {
            recipe_id: recipe.id.clone(),
            recipe_title: recipe.title.clone(),
            risk: recipe.risk.clone(),
            working_dir: base.to_path_buf(),
            variant_id: variant.id.clone(),
            variant_description: variant.description.clone(),
            steps,
        })
    }

    #[cfg(test)]
    #[allow(clippy::result_large_err)]
    fn build_plan(&self, recipe: &Recipe) -> Result<RecipeRunPlan, FailureKind> {
        let base = self
            .env
            .working_directory()
            .map_err(|e| FailureKind::Unavailable {
                reason: e.0,
                retryable: false,
            })?;
        self.build_plan_at(recipe, &base)
    }

    fn result_base(&self, result: &SearchItem) -> Result<PathBuf, FailureKind> {
        if let Some(path) = result
            .action_payload
            .as_ref()
            .and_then(|payload| payload.get("project_path"))
            .and_then(|value| value.as_str())
        {
            return Ok(PathBuf::from(path));
        }
        self.env
            .working_directory()
            .map_err(|error| FailureKind::Unavailable {
                reason: error.0,
                retryable: false,
            })
    }

    async fn query_context(
        &self,
        query: &Query,
    ) -> Result<(PathBuf, String, Option<String>, bool), RecipeQueryContextError> {
        let rest_raw = query.rest_raw().trim();
        let rest_lower = rest_raw.to_lowercase();
        if rest_lower == "project" || rest_lower.starts_with("project ") {
            let path = rest_raw.get("project".len()..).unwrap_or("").trim();
            if path.is_empty() {
                return Err(RecipeQueryContextError {
                    title: "Project recipes unavailable",
                    kind: "not_configured",
                    reason: "Usage: /cmd project /path/to/imported/project".into(),
                });
            }
            let allowed = self
                .projects
                .read()
                .await
                .iter()
                .any(|project| project.path == path);
            if !allowed {
                return Err(RecipeQueryContextError {
                    title: "Project recipes unavailable",
                    kind: "not_configured",
                    reason: "project is not in the imported Projects list".into(),
                });
            }
            return Ok((PathBuf::from(path), String::new(), Some(path.into()), false));
        }
        let base = self
            .env
            .working_directory()
            .map_err(|error| RecipeQueryContextError {
                title: "Command Recipes unavailable",
                kind: "unavailable",
                reason: error.0,
            })?;
        let mut filter = if query.is_command() {
            query
                .normalized
                .split_whitespace()
                .skip(1)
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            query.normalized.clone()
        };
        let show_all = filter == "all" || filter.starts_with("all ");
        if show_all {
            filter = filter
                .strip_prefix("all")
                .unwrap_or_default()
                .trim_start()
                .to_string();
        }
        Ok((base, filter, None, show_all))
    }

    fn copy_text(plan: &RecipeRunPlan) -> String {
        let mut lines = vec![
            format!("# {}", shell_comment(&plan.recipe_title)),
            format!("cd {}", shell_quote(&plan.working_dir.to_string_lossy())),
        ];
        for step in &plan.steps {
            let command = std::iter::once(step.program.as_str())
                .chain(step.args.iter().map(String::as_str))
                .map(shell_quote)
                .collect::<Vec<_>>()
                .join(" ");
            // Comments remain one inert terminal line even if a malformed
            // local recipe contains newlines or terminal control characters.
            let label = shell_comment(&step.label);
            lines.push(format!("{command} # {label}"));
        }
        lines.join("\n")
    }

    fn preview_body(recipe: &Recipe, plan: Option<&RecipeRunPlan>) -> String {
        let mut out = vec![
            format!("Recipe: {}", recipe.id),
            format!("Title: {}", recipe.title),
            format!("Description: {}", recipe.description),
            format!("Risk: {}", recipe.risk.as_str()),
        ];
        if let Some(plan) = plan {
            out.push(format!("Working directory: {}", plan.working_dir.display()));
            out.push(format!(
                "Selected variant: {} — {}",
                plan.variant_id, plan.variant_description
            ));
            for (idx, step) in plan.steps.iter().enumerate() {
                let args = step.args.join(" ");
                out.push(format!(
                    "{}. {} {} {}",
                    idx + 1,
                    step.label,
                    step.program,
                    args
                ));
            }
        } else {
            out.push("Selected variant: (none — 当前项目不适用)".into());
        }
        out.join("\n")
    }
}

/// Render one argument for a POSIX shell without ever changing argument
/// boundaries. Recipes are executed directly (never through a shell); this is
/// only for the intentionally copyable, paste-into-terminal representation.
fn shell_quote(value: &str) -> String {
    if !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_@%+=:,./-".contains(&byte))
    {
        return value.into();
    }
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn shell_comment(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect()
}

#[async_trait]
impl LumaModule for CommandRecipesModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, ctx: WarmupContext) -> ModuleState {
        if await_unless_cancelled(&ctx.cancel, async { self.refresh_catalog() })
            .await
            .is_none()
        {
            return ModuleState::Failed("cancelled".into());
        }
        ModuleState::Ready
    }

    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        if cancel.is_cancelled() {
            return;
        }
        let catalog = self.refresh_catalog();
        if cancel.is_cancelled() {
            return;
        }

        if catalog.has_fatal_issues() {
            let issue = catalog.issues.first().cloned();
            let subtitle = issue
                .as_ref()
                .map(|i| format!("{}: {}", i.location, i.message));
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![SearchItemDto {
                        id: "cmd:unavailable".into(),
                        module_id: MODULE_ID.into(),
                        title: "Command Recipes unavailable".into(),
                        subtitle,
                        kind: "unavailable".into(),
                        score: 100.0,
                        primary_action_id: "noop".into(),
                        primary_action_label: "Unavailable".into(),
                        ..Default::default()
                    }],
                    removed_ids: vec![],
                })
                .await;
            return;
        }

        let (base, filter, project_path, show_all) = match self.query_context(&query).await {
            Ok(context) => context,
            Err(error) => {
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![SearchItemDto {
                            id: "cmd:unavailable".into(),
                            module_id: MODULE_ID.into(),
                            title: error.title.into(),
                            subtitle: Some(error.reason),
                            kind: error.kind.into(),
                            score: 100.0,
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

        let mut upserts = Vec::new();
        for recipe in catalog.recipes.iter().filter(|r| r.enabled) {
            if !self.scope_visible(recipe, &base) {
                continue;
            }
            let meta = self.repo.get_metadata(&recipe.id).unwrap_or_default();
            let score = Self::score_recipe(recipe, &filter, &meta);
            if !filter.is_empty() && score < 60.0 {
                continue;
            }
            let (matched, variant_id) = match self.env.match_variant(&base, &recipe.variants) {
                VariantMatch::Matched(v) => (true, Some(v.id.clone())),
                VariantMatch::NoMatch => (false, None),
            };
            if !matched && !show_all && (filter.is_empty() || score < 90.0) {
                continue;
            }
            let kind = if matched { "recipe" } else { "no_match" };
            // Targeted result stores sort by score rather than emission order. Keep runnable
            // recipes in their normal relevance bands and move explicitly requested incompatible
            // definitions below them; global search filters `no_match` rows entirely.
            let display_score = if matched { score } else { score - 1_000.0 };
            let primary = ActionDescriptor {
                id: ActionId::new("preview"),
                label: "Preview".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            };
            upserts.push(SearchItemDto {
                id: format!("cmd:{}", recipe.id),
                module_id: MODULE_ID.into(),
                title: recipe.title.clone(),
                subtitle: Some(Self::format_subtitle(
                    recipe,
                    &meta,
                    variant_id.as_deref(),
                    matched,
                )),
                kind: kind.into(),
                score: display_score,
                primary_action_id: primary.id.as_str().to_string(),
                primary_action_label: primary.label.clone(),
                primary_action_risk: primary.risk.clone(),
                primary_action_confirmation: primary.confirmation,
                secondary_actions: vec![],
                action_payload: Some(serde_json::json!({
                    "recipe_id": recipe.id,
                    "matched": matched,
                    "variant_id": variant_id,
                    "project_path": project_path,
                })),
                ..Default::default()
            });
        }

        upserts.sort_by(|a, b| {
            let a_matched = a.kind != "no_match";
            let b_matched = b.kind != "no_match";
            b_matched
                .cmp(&a_matched)
                .then_with(|| {
                    b.score
                        .partial_cmp(&a.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| a.title.cmp(&b.title))
        });

        let warnings: Vec<_> = catalog.warnings().cloned().collect();
        if !warnings.is_empty() {
            let subtitle = warnings
                .iter()
                .map(|issue| format!("{}: {}", issue.location, issue.message))
                .collect::<Vec<_>>()
                .join(" · ");
            upserts.push(SearchItemDto {
                id: "cmd:config-warning".into(),
                module_id: MODULE_ID.into(),
                title: "Command Recipes config warning".into(),
                subtitle: Some(subtitle),
                kind: "warning".into(),
                score: 1.0,
                primary_action_id: "noop".into(),
                primary_action_label: "Warning".into(),
                ..Default::default()
            });
        }

        if upserts.is_empty() {
            upserts.push(SearchItemDto {
                id: "cmd:no-match".into(),
                module_id: MODULE_ID.into(),
                title: "No matching command recipes".into(),
                subtitle: Some(
                    project_path
                        .as_ref()
                        .map(|path| format!("No runnable recipes for {path}"))
                        .unwrap_or_else(|| {
                            "Try another query or configure a project variant".into()
                        }),
                ),
                kind: "status".into(),
                score: 0.0,
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
        if result.kind == "unavailable" || result.id.as_str() == "cmd:unavailable" {
            return vec![ActionDescriptor {
                id: ActionId::new("noop"),
                label: "Unavailable".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        let Some(recipe_id) = Self::recipe_id_from_result(result.id.as_str()) else {
            return vec![];
        };
        let catalog = self.repo.load_catalog();
        let Some(recipe) = catalog.recipe_by_id(&recipe_id) else {
            return vec![];
        };
        let matched = self.result_base(result).is_ok_and(|base| {
            matches!(
                self.env.match_variant(&base, &recipe.variants),
                VariantMatch::Matched(_)
            )
        });
        let meta = self.repo.get_metadata(&recipe_id).unwrap_or_default();
        let run_risk = Self::risk_to_action(&recipe.risk);
        let mut actions = vec![
            ActionDescriptor {
                id: ActionId::new("preview"),
                label: "Preview".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            ActionDescriptor {
                id: ActionId::new("copy"),
                label: "Copy".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            ActionDescriptor {
                id: ActionId::new(if meta.favorite {
                    "unfavorite"
                } else {
                    "favorite"
                }),
                label: if meta.favorite {
                    "Unfavorite".into()
                } else {
                    "Favorite".into()
                },
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            ActionDescriptor {
                id: ActionId::new("open_config"),
                label: "Open config".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            ActionDescriptor {
                id: ActionId::new("show_variant"),
                label: "Show variant".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
        ];
        if matched {
            actions.insert(
                1,
                ActionDescriptor {
                    id: ActionId::new("run"),
                    label: "Run".into(),
                    risk: run_risk.clone(),
                    confirmation: !matches!(run_risk, ActionRisk::Safe),
                },
            );
        }
        actions
    }

    async fn preview(&self, result: &SearchItem) -> Option<String> {
        let recipe_id = Self::recipe_id_from_result(result.id.as_str())?;
        let catalog = self.repo.load_catalog();
        let recipe = catalog.recipe_by_id(&recipe_id)?;
        let plan = self
            .result_base(result)
            .and_then(|base| self.build_plan_at(recipe, &base))
            .ok();
        Some(Self::preview_body(recipe, plan.as_ref()))
    }

    async fn perform(&self, request: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        let recipe_id = match Self::recipe_id_from_result(request.result.id.as_str()) {
            Some(id) => id,
            None => {
                return ActionOutcome::Failed {
                    kind: FailureKind::NotFound {
                        entity: request.result.id.as_str().to_string(),
                    },
                }
            }
        };
        let catalog = self.repo.load_catalog();
        let Some(recipe) = catalog.recipe_by_id(&recipe_id).cloned() else {
            return ActionOutcome::Failed {
                kind: FailureKind::NotFound {
                    entity: format!("recipe:{recipe_id}"),
                },
            };
        };

        let base = match self.result_base(&request.result) {
            Ok(base) => base,
            Err(kind) => return ActionOutcome::Failed { kind },
        };
        if let Some(project_path) = request
            .result
            .action_payload
            .as_ref()
            .and_then(|payload| payload.get("project_path"))
            .and_then(|value| value.as_str())
        {
            if !self
                .projects
                .read()
                .await
                .iter()
                .any(|project| project.path == project_path)
            {
                return ActionOutcome::Failed {
                    kind: FailureKind::SecurityDenied {
                        reason: "project is no longer imported".into(),
                    },
                };
            }
        }

        match request.action.id.as_str() {
            "preview" | "show_variant" => ActionOutcome::Success {
                message: Some(Self::preview_body(
                    &recipe,
                    self.build_plan_at(&recipe, &base).ok().as_ref(),
                )),
            },
            "copy" => match self.build_plan_at(&recipe, &base) {
                Ok(plan) => {
                    match await_unless_cancelled(
                        &cancel,
                        self.pasteboard.write_text(&Self::copy_text(&plan)),
                    )
                    .await
                    {
                        None => ActionOutcome::Cancelled,
                        Some(Ok(())) => ActionOutcome::Success {
                            message: Some("copied recipe commands".into()),
                        },
                        Some(Err(err)) => ActionOutcome::Failed {
                            kind: FailureKind::Unavailable {
                                reason: err.to_string(),
                                retryable: false,
                            },
                        },
                    }
                }
                Err(kind) => ActionOutcome::Failed { kind },
            },
            "favorite" => {
                if self.repo.set_favorite(&recipe_id, true).is_err() {
                    return ActionOutcome::Failed {
                        kind: FailureKind::Io {
                            context: "favorite".into(),
                        },
                    };
                }
                ActionOutcome::Success {
                    message: Some("favorited".into()),
                }
            }
            "unfavorite" => {
                if self.repo.set_favorite(&recipe_id, false).is_err() {
                    return ActionOutcome::Failed {
                        kind: FailureKind::Io {
                            context: "unfavorite".into(),
                        },
                    };
                }
                ActionOutcome::Success {
                    message: Some("unfavorited".into()),
                }
            }
            "open_config" => {
                if let Some(path) = self.repo.config_path() {
                    match self.opener.open(&path).await {
                        Ok(()) => ActionOutcome::Success {
                            message: Some("opened config".into()),
                        },
                        Err(err) => ActionOutcome::Failed {
                            kind: FailureKind::Unavailable {
                                reason: err.to_string(),
                                retryable: false,
                            },
                        },
                    }
                } else {
                    ActionOutcome::Failed {
                        kind: FailureKind::NotConfigured {
                            remediation: "command-recipes.toml path unavailable".into(),
                        },
                    }
                }
            }
            "run" => match self.build_plan_at(&recipe, &base) {
                Ok(plan) => ActionOutcome::InteractiveRecipeRun {
                    plan: Box::new(plan),
                },
                Err(kind) => ActionOutcome::Failed { kind },
            },
            _ => ActionOutcome::Failed {
                kind: FailureKind::NotFound {
                    entity: format!("action:{}", request.action.id.as_str()),
                },
            },
        }
    }

    async fn teardown(&self) {}

    async fn apply_settings(&self, settings: &AppSettings) {
        *self.projects.write().await = settings.imported_projects.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_application::{
        FakeOpenPath, FakePasteboard, FakeRecipeEnvironment, MemoryCommandRecipesRepository,
    };
    use luma_storage::builtin_recipes;
    use std::path::PathBuf;

    fn test_module(env: FakeRecipeEnvironment) -> CommandRecipesModule {
        let catalog = luma_domain::RecipeCatalog {
            recipes: builtin_recipes(),
            issues: vec![],
            config_path: None,
        };
        CommandRecipesModule::with_deps(
            Arc::new(MemoryCommandRecipesRepository::with_catalog(catalog)),
            Arc::new(env),
            Arc::new(FakePasteboard::new()),
            Arc::new(FakeOpenPath::new()),
        )
    }

    async fn search_rows(module: &CommandRecipesModule, prompt: &str) -> Vec<SearchItemDto> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(8);
        module
            .search(Query::parse(prompt, 100), tx, CancellationToken::new())
            .await;
        let Event::ResultsChunk { upserts, .. } = rx.recv().await.unwrap() else {
            panic!("expected results");
        };
        upserts
    }

    #[tokio::test]
    async fn cargo_project_matches_rust_test_variant() {
        let env = FakeRecipeEnvironment::new("/proj");
        env.add_file(PathBuf::from("/proj/Cargo.toml"));
        env.add_command("cargo");
        let module = test_module(env);
        let recipe = builtin_recipes()
            .into_iter()
            .find(|r| r.id == "test")
            .unwrap();
        let plan = module.build_plan(&recipe).unwrap();
        assert_eq!(plan.variant_id, "rust");
        assert_eq!(plan.steps[0].program, "cargo");
    }

    #[tokio::test]
    async fn no_match_returns_invalid_input_on_run() {
        let module = test_module(FakeRecipeEnvironment::new("/empty"));
        let outcome = module
            .perform(
                ActionRequest {
                    result: SearchItem {
                        id: luma_domain::ResultId::new("cmd:test"),
                        module_id: ModuleId::new(MODULE_ID),
                        title: "test".into(),
                        subtitle: None,
                        kind: "recipe".into(),
                        score: 1.0,
                        primary_action: ActionDescriptor {
                            id: ActionId::new("run"),
                            label: "Run".into(),
                            risk: ActionRisk::Safe,
                            confirmation: false,
                        },
                        secondary_actions: vec![],
                        ui_intent: None,
                        action_payload: None,
                    },
                    action: ActionDescriptor {
                        id: ActionId::new("run"),
                        label: "Run".into(),
                        risk: ActionRisk::Safe,
                        confirmation: false,
                    },
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(
            outcome,
            ActionOutcome::Failed {
                kind: FailureKind::InvalidInput { .. }
            }
        ));
    }

    #[tokio::test]
    async fn default_recipe_surface_hides_inapplicable_catalogue_rows() {
        let module = test_module(FakeRecipeEnvironment::new("/empty"));
        let rows = search_rows(&module, "/cmd ").await;
        assert!(rows.iter().all(|row| row.kind != "no_match"));

        let all_rows = search_rows(&module, "/cmd all").await;
        assert!(all_rows.iter().any(|row| row.kind == "no_match"));
    }

    #[tokio::test]
    async fn all_surface_orders_runnable_recipes_before_inapplicable_ones() {
        let env = FakeRecipeEnvironment::new("/proj");
        env.add_file(PathBuf::from("/proj/Cargo.toml"));
        env.add_command("cargo");
        let module = test_module(env);
        let rows = search_rows(&module, "/cmd all").await;
        let first_no_match = rows
            .iter()
            .position(|row| row.kind == "no_match")
            .expect("catalogue contains non-Rust recipes");
        assert!(rows[..first_no_match]
            .iter()
            .all(|row| row.kind == "recipe"));
        assert!(rows[first_no_match..]
            .iter()
            .all(|row| row.kind == "no_match"));
    }

    #[tokio::test]
    async fn git_recipes_match_regular_repositories_and_worktrees() {
        let regular = FakeRecipeEnvironment::new("/repo");
        regular.add_directory(PathBuf::from("/repo/.git"));
        regular.add_command("git");
        let regular_rows = search_rows(&test_module(regular), "/cmd git").await;
        let regular_git = regular_rows
            .iter()
            .filter(|row| row.id.starts_with("cmd:git-"))
            .collect::<Vec<_>>();
        assert!(!regular_git.is_empty());
        assert!(regular_git.iter().all(|row| row.kind == "recipe"));

        let worktree = FakeRecipeEnvironment::new("/worktree");
        worktree.add_file(PathBuf::from("/worktree/.git"));
        worktree.add_command("git");
        let worktree_rows = search_rows(&test_module(worktree), "/cmd git").await;
        let worktree_git = worktree_rows
            .iter()
            .filter(|row| row.id.starts_with("cmd:git-"))
            .collect::<Vec<_>>();
        assert!(!worktree_git.is_empty());
        assert!(worktree_git.iter().all(|row| row.kind == "recipe"));
    }

    #[tokio::test]
    async fn imported_project_surface_resolves_recipes_against_that_project() {
        let env = FakeRecipeEnvironment::new("/current");
        env.add_file(PathBuf::from("/workspace/app/Cargo.toml"));
        env.add_command("cargo");
        let module = test_module(env).with_projects(vec![ImportedProject {
            path: "/workspace/app".into(),
            name: Some("app".into()),
        }]);
        let (tx, mut rx) = tokio::sync::mpsc::channel(8);
        module
            .search(
                Query::parse("cmd project /workspace/app", 50),
                tx,
                CancellationToken::new(),
            )
            .await;
        let Event::ResultsChunk { upserts, .. } = rx.recv().await.unwrap() else {
            panic!("expected results");
        };
        let test = upserts
            .iter()
            .find(|item| item.id == "cmd:test")
            .expect("Rust test recipe");
        assert_eq!(test.kind, "recipe");
        assert_eq!(
            test.action_payload
                .as_ref()
                .and_then(|payload| payload.get("project_path"))
                .and_then(|value| value.as_str()),
            Some("/workspace/app")
        );
        let outcome = module
            .perform(
                ActionRequest {
                    result: test.clone().into_domain(),
                    action: ActionDescriptor {
                        id: ActionId::new("run"),
                        label: "Run".into(),
                        risk: ActionRisk::Safe,
                        confirmation: false,
                    },
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        match outcome {
            ActionOutcome::InteractiveRecipeRun { plan } => {
                assert_eq!(plan.working_dir, PathBuf::from("/workspace/app"));
                assert_eq!(plan.steps[0].cwd, PathBuf::from("/workspace/app"));
            }
            other => panic!("expected project recipe plan, got {other:?}"),
        }
    }

    #[test]
    fn copy_text_uses_program_and_args() {
        let plan = RecipeRunPlan {
            recipe_id: "x".into(),
            recipe_title: "X".into(),
            risk: RecipeRisk::Safe,
            working_dir: PathBuf::from("/tmp"),
            variant_id: "v".into(),
            variant_description: "v".into(),
            steps: vec![luma_domain::ResolvedCommandStep {
                id: "s".into(),
                label: "cargo test".into(),
                program: "cargo".into(),
                args: vec!["test".into()],
                cwd: PathBuf::from("/tmp"),
                root: PathBuf::from("/tmp"),
                continue_on_error: false,
            }],
        };
        let text = CommandRecipesModule::copy_text(&plan);
        assert!(text.contains("cargo test"));
        assert!(!text.contains("sh -c"));
    }

    #[test]
    fn copy_text_quotes_paths_and_shell_metacharacters() {
        let plan = RecipeRunPlan {
            recipe_id: "x".into(),
            recipe_title: "X\nprintf injected".into(),
            risk: RecipeRisk::Safe,
            working_dir: PathBuf::from("/tmp/a folder"),
            variant_id: "v".into(),
            variant_description: "v".into(),
            steps: vec![luma_domain::ResolvedCommandStep {
                id: "s".into(),
                label: "echo\nlabel\u{1b}[31m".into(),
                program: "echo".into(),
                args: vec![
                    "hello world".into(),
                    "$(not-a-command)".into(),
                    "it's".into(),
                ],
                cwd: PathBuf::from("/tmp/a folder"),
                root: PathBuf::from("/tmp/a folder"),
                continue_on_error: false,
            }],
        };

        let text = CommandRecipesModule::copy_text(&plan);

        assert!(text.contains("cd '/tmp/a folder'"));
        assert!(text.starts_with("# X printf injected\n"));
        assert!(
            text.contains("echo 'hello world' '$(not-a-command)' 'it'\"'\"'s' # echo label [31m")
        );
        assert!(!text.contains('\u{1b}'));
        assert_eq!(text.lines().count(), plan.steps.len() + 2);
    }
}
