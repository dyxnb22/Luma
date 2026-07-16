use crate::cancel::await_unless_cancelled;
use async_trait::async_trait;
use luma_application::{
    resolve_steps, ActionOutcome, ActionRequest, CommandRecipesRepository, LumaModule,
    ModuleManifest, ModuleState, OpenPathPort, PasteboardPort, RecipeEnvironmentPort, SearchMode,
    SearchSink, WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, Recipe, RecipeMetadata,
    RecipeRisk, RecipeRunPlan, RecipeScope, SearchItem, VariantMatch,
};
use luma_protocol::{Event, SearchItemDto};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

const MODULE_ID: &str = "luma.command_recipes";

pub struct CommandRecipesModule {
    manifest: ModuleManifest,
    repo: Arc<dyn CommandRecipesRepository>,
    env: Arc<dyn RecipeEnvironmentPort>,
    pasteboard: Arc<dyn PasteboardPort>,
    opener: Arc<dyn OpenPathPort>,
    catalog_error: RwLock<Option<String>>,
}

impl CommandRecipesModule {
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
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
                workbench: luma_application::WorkbenchMeta {
                    glyph: Some("C".into()),
                    suggested_query: Some("cmd ".into()),
                    empty_hint: Some("cmd · recipe test · r run · c copy · f favorite".into()),
                    supports_browse: false,
                },
            },
            repo,
            env,
            pasteboard,
            opener,
            catalog_error: RwLock::new(None),
        }
    }

    async fn refresh_catalog(&self) -> luma_domain::RecipeCatalog {
        let catalog = self.repo.load_catalog();
        if catalog.issues.is_empty() {
            *self.catalog_error.write().await = None;
        } else {
            let msg = catalog
                .issues
                .iter()
                .map(|i| format!("{}: {}", i.location, i.message))
                .collect::<Vec<_>>()
                .join(" · ");
            *self.catalog_error.write().await = Some(msg);
        }
        catalog
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
        match recipe.scope {
            RecipeScope::Global => true,
            RecipeScope::CurrentProject => true,
            RecipeScope::LumaRepository => self.env.is_luma_repository(base),
        }
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
    fn build_plan(&self, recipe: &Recipe) -> Result<RecipeRunPlan, FailureKind> {
        let base = self
            .env
            .working_directory()
            .map_err(|e| FailureKind::Unavailable {
                reason: e.0,
                retryable: false,
            })?;
        let variant = match self.env.match_variant(&base, &recipe.variants) {
            VariantMatch::Matched(v) => v,
            VariantMatch::NoMatch => {
                return Err(FailureKind::InvalidInput {
                    field: "variant".into(),
                    message: "当前项目不适用".into(),
                });
            }
        };
        let steps = resolve_steps(self.env.as_ref(), &base, &variant).map_err(|e| {
            FailureKind::InvalidInput {
                field: "cwd".into(),
                message: e.0,
            }
        })?;
        Ok(RecipeRunPlan {
            recipe_id: recipe.id.clone(),
            recipe_title: recipe.title.clone(),
            risk: recipe.risk.clone(),
            working_dir: base,
            variant_id: variant.id.clone(),
            variant_description: variant.description.clone(),
            steps,
        })
    }

    fn copy_text(plan: &RecipeRunPlan) -> String {
        let mut lines = vec![
            format!("# {}", plan.recipe_title),
            format!("cd {}", plan.working_dir.display()),
        ];
        for step in &plan.steps {
            let args = step.args.join(" ");
            if args.is_empty() {
                lines.push(format!("{} # {}", step.program, step.label));
            } else {
                lines.push(format!("{} {} # {}", step.program, args, step.label));
            }
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

#[async_trait]
impl LumaModule for CommandRecipesModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, ctx: WarmupContext) -> ModuleState {
        if await_unless_cancelled(&ctx.cancel, self.refresh_catalog())
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
        let catalog = self.refresh_catalog().await;
        if cancel.is_cancelled() {
            return;
        }

        if !catalog.issues.is_empty() {
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

        let base = match self.env.working_directory() {
            Ok(p) => p,
            Err(e) => {
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![SearchItemDto {
                            id: "cmd:unavailable".into(),
                            module_id: MODULE_ID.into(),
                            title: "Command Recipes unavailable".into(),
                            subtitle: Some(e.0),
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
        };

        let filter = query
            .normalized
            .split_whitespace()
            .skip(1)
            .collect::<Vec<_>>()
            .join(" ");

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
            let matched = matches!(
                self.env.match_variant(&base, &recipe.variants),
                VariantMatch::Matched(_)
            );
            let variant_id = match self.env.match_variant(&base, &recipe.variants) {
                VariantMatch::Matched(v) => Some(v.id.clone()),
                VariantMatch::NoMatch => None,
            };
            let kind = if matched { "recipe" } else { "no_match" };
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
                score,
                primary_action_id: primary.id.as_str().to_string(),
                primary_action_label: primary.label.clone(),
                primary_action_risk: primary.risk.clone(),
                primary_action_confirmation: primary.confirmation,
                secondary_actions: vec![],
                action_payload: Some(serde_json::json!({
                    "recipe_id": recipe.id,
                    "matched": matched,
                    "variant_id": variant_id,
                })),
                ..Default::default()
            });
        }

        upserts.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

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
        let matched = result
            .action_payload
            .as_ref()
            .and_then(|p| p.get("matched"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
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
        let plan = self.build_plan(recipe).ok();
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

        match request.action.id.as_str() {
            "preview" | "show_variant" => ActionOutcome::Success {
                message: Some(Self::preview_body(
                    &recipe,
                    self.build_plan(&recipe).ok().as_ref(),
                )),
            },
            "copy" => match self.build_plan(&recipe) {
                Ok(plan) => match self.pasteboard.write_text(&Self::copy_text(&plan)).await {
                    Ok(()) => ActionOutcome::Success {
                        message: Some("copied recipe commands".into()),
                    },
                    Err(err) => ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: err.to_string(),
                            retryable: false,
                        },
                    },
                },
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
            "run" => match self.build_plan(&recipe) {
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
                continue_on_error: false,
            }],
        };
        let text = CommandRecipesModule::copy_text(&plan);
        assert!(text.contains("cargo test"));
        assert!(!text.contains("sh -c"));
    }
}
