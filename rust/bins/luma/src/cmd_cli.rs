use crate::compose::load_registry_with_settings;
use luma_application::{
    execute_recipe_plan_with_hooks, now_unix, recipe_in_scope, recipe_runnable,
    record_recipe_run_outcome, resolve_steps, run_action, spawn_ctrl_c_cancel,
    CommandRecipesRepository, CommandRunnerPort, RecipeEnvironmentPort, RecipeExecuteOptions,
    RecipeStdioMode,
};
use luma_domain::{RecipeRunOutcome, RecipeRunPlan, VariantMatch};
use luma_platform_macos::{MacCommandRunner, MacRecipeEnvironment};
use luma_protocol::ActionOutcomeDto;
use serde_json::json;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[derive(Debug, clap::Subcommand)]
pub enum CmdCmd {
    List {
        #[arg(long)]
        json: bool,
    },
    Show {
        recipe_id: String,
        #[arg(long)]
        json: bool,
    },
    Run {
        recipe_id: String,
        #[arg(long)]
        confirmation: bool,
        #[arg(long)]
        json: bool,
    },
    Copy {
        recipe_id: String,
        #[arg(long)]
        json: bool,
    },
}

pub async fn handle_cmd_command(action: CmdCmd) -> anyhow::Result<()> {
    let load = load_registry_with_settings()?;
    let repo = load
        .command_recipes
        .ok_or_else(|| anyhow::anyhow!("command recipes repository unavailable"))?;
    let catalog = repo.load_catalog();
    if catalog.has_fatal_issues() {
        let issue = catalog
            .issues
            .iter()
            .find(|issue| issue.fatal)
            .map(|issue| format!("{}: {}", issue.location, issue.message))
            .unwrap_or_else(|| "command recipes config unavailable".into());
        anyhow::bail!(issue);
    }
    let env = Arc::new(MacRecipeEnvironment::new());
    let runner = Arc::new(MacCommandRunner::new()) as Arc<dyn CommandRunnerPort>;

    match action {
        CmdCmd::List { json } => cmd_list(repo.as_ref(), env.as_ref(), json),
        CmdCmd::Show { recipe_id, json } => cmd_show(repo.as_ref(), env.as_ref(), &recipe_id, json),
        CmdCmd::Run {
            recipe_id,
            confirmation,
            json,
        } => {
            cmd_run(
                repo.as_ref(),
                env.as_ref(),
                runner.as_ref(),
                &recipe_id,
                confirmation,
                json,
            )
            .await
        }
        CmdCmd::Copy { recipe_id, json } => cmd_copy(&recipe_id, json).await,
    }
}

fn cmd_list(
    repo: &dyn CommandRecipesRepository,
    env: &dyn RecipeEnvironmentPort,
    json: bool,
) -> anyhow::Result<()> {
    let catalog = repo.load_catalog();
    let base = env.working_directory().map_err(|e| anyhow::anyhow!(e.0))?;
    let rows: Vec<_> = catalog
        .recipes
        .iter()
        .filter(|r| r.enabled)
        .filter(|recipe| recipe_in_scope(env, &base, &recipe.scope))
        .map(|recipe| {
            let matched = matches!(
                env.match_variant(&base, &recipe.variants),
                VariantMatch::Matched(_)
            );
            let meta = repo.get_metadata(&recipe.id).unwrap_or_default();
            json!({
                "id": recipe.id,
                "title": recipe.title,
                "risk": recipe.risk.as_str(),
                "matched": matched,
                "favorite": meta.favorite,
                "use_count": meta.use_count,
            })
        })
        .collect();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "recipes": rows }))?
        );
    } else {
        for row in &rows {
            println!(
                "{} — {} (risk: {}, matched: {})",
                row["id"], row["title"], row["risk"], row["matched"]
            );
        }
    }
    Ok(())
}

fn cmd_show(
    repo: &dyn CommandRecipesRepository,
    env: &dyn RecipeEnvironmentPort,
    recipe_id: &str,
    json: bool,
) -> anyhow::Result<()> {
    let catalog = repo.load_catalog();
    let recipe = catalog
        .recipe_by_id(recipe_id)
        .ok_or_else(|| anyhow::anyhow!("recipe not found: {recipe_id}"))?;
    let base = env.working_directory().map_err(|e| anyhow::anyhow!(e.0))?;
    recipe_runnable(env, &base, recipe).map_err(|e| anyhow::anyhow!(e))?;
    let variant = env.match_variant(&base, &recipe.variants);
    let payload = match variant {
        VariantMatch::Matched(v) => {
            let steps = resolve_steps(env, &base, &v).map_err(|e| anyhow::anyhow!(e.0))?;
            json!({
                "recipe": recipe,
                "variant": v.id,
                "working_dir": base,
                "steps": steps,
            })
        }
        VariantMatch::NoMatch => json!({
            "recipe": recipe,
            "error": "当前项目不适用",
        }),
    };
    // Show and list both emit JSON today (human table not wired yet).
    let _ = json;
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

async fn cmd_run(
    repo: &dyn CommandRecipesRepository,
    env: &dyn RecipeEnvironmentPort,
    runner: &dyn CommandRunnerPort,
    recipe_id: &str,
    confirmation: bool,
    json: bool,
) -> anyhow::Result<()> {
    let catalog = repo.load_catalog();
    let recipe = catalog
        .recipe_by_id(recipe_id)
        .ok_or_else(|| anyhow::anyhow!("recipe not found: {recipe_id}"))?;
    let base = env.working_directory().map_err(|e| anyhow::anyhow!(e.0))?;
    recipe_runnable(env, &base, recipe).map_err(|e| anyhow::anyhow!(e))?;
    let variant = match env.match_variant(&base, &recipe.variants) {
        VariantMatch::Matched(v) => v,
        VariantMatch::NoMatch => anyhow::bail!("当前项目不适用"),
    };
    let steps = resolve_steps(env, &base, &variant).map_err(|e| anyhow::anyhow!(e.0))?;
    let plan = RecipeRunPlan {
        recipe_id: recipe.id.clone(),
        recipe_title: recipe.title.clone(),
        risk: recipe.risk.clone(),
        working_dir: base.clone(),
        variant_id: variant.id.clone(),
        variant_description: variant.description.clone(),
        steps,
    };

    let stdio = if json {
        RecipeStdioMode::Null
    } else {
        RecipeStdioMode::Inherit
    };
    if !json {
        println!("Running recipe: {} — {}", plan.recipe_title, plan.recipe_id);
        println!("Risk: {}", plan.risk.as_str());
        println!("Working directory: {}", plan.working_dir.display());
    }

    let cancel = CancellationToken::new();
    let cancel_task = spawn_ctrl_c_cancel(cancel.clone());
    let report = execute_recipe_plan_with_hooks(
        &plan,
        runner,
        &cancel,
        RecipeExecuteOptions {
            confirmation,
            stdio,
        },
        |step| {
            if !json {
                println!("\n→ {}", step.label);
            }
        },
        |_, result| {
            if !json {
                if result.cancelled {
                    println!("cancelled");
                } else if let Some(code) = result.exit_code {
                    println!("exit code: {code}");
                } else if !result.started {
                    eprintln!("{}", result.message.as_deref().unwrap_or("failed to start"));
                }
            }
        },
    );
    cancel_task.abort();

    let report = match report {
        Ok(report) => report,
        Err(err) => anyhow::bail!(err),
    };

    record_recipe_run_outcome(repo, &plan.recipe_id, report.outcome.clone(), now_unix());

    let exit = match report.outcome {
        RecipeRunOutcome::Success => 0,
        RecipeRunOutcome::Failed => 1,
        RecipeRunOutcome::Cancelled => 2,
    };

    if json {
        let mut payload = json!({
            "recipe_id": plan.recipe_id,
            "outcome": match report.outcome {
                RecipeRunOutcome::Success => "success",
                RecipeRunOutcome::Failed => "failed",
                RecipeRunOutcome::Cancelled => "cancelled",
            },
        });
        if let Some(code) = report.last_exit_code {
            payload["exit_code"] = json!(code);
        }
        if let Some(message) = &report.message {
            payload["error"] = json!(message);
        }
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else if matches!(report.outcome, RecipeRunOutcome::Cancelled) {
        eprintln!("recipe cancelled");
    }

    if exit != 0 {
        std::process::exit(exit);
    }
    Ok(())
}

async fn cmd_copy(recipe_id: &str, json: bool) -> anyhow::Result<()> {
    let load = load_registry_with_settings()?;
    let (item, outcome) = run_action(
        load.registry,
        &format!("cmd {recipe_id}"),
        None,
        "copy",
        false,
        luma_application::RunActionOptions {
            settings: Some(load.settings),
            ..Default::default()
        },
    )
    .await
    .map_err(|e| anyhow::anyhow!(e))?;
    if json {
        let payload = match outcome {
            ActionOutcomeDto::Success { message } => {
                json!({ "result_id": item.id, "message": message })
            }
            ActionOutcomeDto::Failed { kind, .. } => json!({ "error": kind.display_message() }),
            ActionOutcomeDto::Cancelled => json!({ "cancelled": true }),
            ActionOutcomeDto::InteractiveRecipeRun { .. } => {
                json!({ "error": "unexpected interactive run" })
            }
            ActionOutcomeDto::InteractiveTerminal { .. } => {
                json!({ "error": "unexpected interactive terminal" })
            }
        };
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else if let ActionOutcomeDto::Success { message } = outcome {
        println!("{}", message.unwrap_or_else(|| "copied".into()));
    }
    Ok(())
}
