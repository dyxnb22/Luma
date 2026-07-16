use crate::compose::load_registry_with_settings;
use luma_application::{
    recipe_in_scope, recipe_runnable, resolve_steps, run_action, CommandRecipesRepository,
    CommandRunnerPort, RecipeEnvironmentPort,
};
use luma_domain::{RecipeRisk, RecipeRunOutcome, VariantMatch};
use luma_platform_macos::{MacCommandRunner, MacRecipeEnvironment};
use luma_protocol::ActionOutcomeDto;
use serde_json::json;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::warn;

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
    let needs_confirm = !matches!(recipe.risk, RecipeRisk::Safe);
    if needs_confirm && !confirmation {
        anyhow::bail!(
            "recipe `{}` requires --confirmation (risk: {})",
            recipe.id,
            recipe.risk.as_str()
        );
    }

    if !json {
        println!("Running recipe: {} — {}", recipe.title, recipe.id);
        println!("Risk: {}", recipe.risk.as_str());
        println!("Working directory: {}", base.display());
    }
    for step in &steps {
        if !json {
            println!("\n→ {}", step.label);
        }
        let result = runner.run_step(step, &tokio_util::sync::CancellationToken::new());
        if !result.started {
            if let Err(err) = repo.record_run(&recipe.id, RecipeRunOutcome::Failed, now_unix()) {
                warn!(recipe_id = %recipe.id, error = %err, "failed to record recipe run");
            }
            let message = result.message.unwrap_or_else(|| "failed to start".into());
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "recipe_id": recipe.id,
                        "outcome": "failed",
                        "error": message,
                    }))?
                );
            } else {
                eprintln!("{message}");
            }
            std::process::exit(1);
        }
        if let Some(code) = result.exit_code {
            if code != 0 && !step.continue_on_error {
                if let Err(err) = repo.record_run(&recipe.id, RecipeRunOutcome::Failed, now_unix())
                {
                    warn!(recipe_id = %recipe.id, error = %err, "failed to record recipe run");
                }
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "recipe_id": recipe.id,
                            "outcome": "failed",
                            "exit_code": code,
                        }))?
                    );
                }
                std::process::exit(1);
            }
        } else {
            if let Err(err) = repo.record_run(&recipe.id, RecipeRunOutcome::Failed, now_unix()) {
                warn!(recipe_id = %recipe.id, error = %err, "failed to record recipe run");
            }
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "recipe_id": recipe.id,
                        "outcome": "failed",
                        "error": "process terminated by signal",
                    }))?
                );
            } else {
                eprintln!("process terminated by signal");
            }
            std::process::exit(1);
        }
    }
    if let Err(err) = repo.record_run(&recipe.id, RecipeRunOutcome::Success, now_unix()) {
        warn!(recipe_id = %recipe.id, error = %err, "failed to record recipe run");
    }
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "recipe_id": recipe.id,
                "outcome": "success",
            }))?
        );
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
        Some(load.settings),
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

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_secs()).unwrap_or(0))
        .unwrap_or(0)
}
