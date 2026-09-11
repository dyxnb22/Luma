//! Shared recipe execution for CLI (`cmd run` / `action run`) and TUI.

use crate::ports::{CommandRecipesRepository, CommandRunnerPort, RecipeStdioMode};
use luma_domain::{
    RecipeRisk, RecipeRunOutcome, RecipeRunPlan, ResolvedCommandStep, StepRunResult,
};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tokio_util::sync::CancellationToken;
use tracing::warn;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecipeExecuteOptions {
    pub confirmation: bool,
    pub stdio: RecipeStdioMode,
}

impl Default for RecipeExecuteOptions {
    fn default() -> Self {
        Self {
            confirmation: false,
            stdio: RecipeStdioMode::Inherit,
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RecipeExecuteError {
    #[error("recipe `{recipe_id}` requires --confirmation (risk: {risk})")]
    ConfirmationRequired { recipe_id: String, risk: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipeExecuteReport {
    pub outcome: RecipeRunOutcome,
    pub last_step_id: Option<String>,
    pub last_exit_code: Option<i32>,
    pub message: Option<String>,
}

/// Risk gate + ordered step loop. Does not persist metadata.
pub fn execute_recipe_plan(
    plan: &RecipeRunPlan,
    runner: &dyn CommandRunnerPort,
    cancel: &CancellationToken,
    opts: RecipeExecuteOptions,
) -> Result<RecipeExecuteReport, RecipeExecuteError> {
    execute_recipe_plan_with_hooks(plan, runner, cancel, opts, |_| {}, |_, _| {})
}

/// Same as [`execute_recipe_plan`] with optional step hooks for human-readable progress.
pub fn execute_recipe_plan_with_hooks(
    plan: &RecipeRunPlan,
    runner: &dyn CommandRunnerPort,
    cancel: &CancellationToken,
    opts: RecipeExecuteOptions,
    mut before_step: impl FnMut(&ResolvedCommandStep),
    mut after_step: impl FnMut(&ResolvedCommandStep, &StepRunResult),
) -> Result<RecipeExecuteReport, RecipeExecuteError> {
    if !matches!(plan.risk, RecipeRisk::Safe) && !opts.confirmation {
        return Err(RecipeExecuteError::ConfirmationRequired {
            recipe_id: plan.recipe_id.clone(),
            risk: plan.risk.as_str().to_string(),
        });
    }

    let mut outcome = RecipeRunOutcome::Success;
    let mut last_step_id = None;
    let mut last_exit_code = None;
    let mut message = None;

    for step in &plan.steps {
        if cancel.is_cancelled() {
            outcome = RecipeRunOutcome::Cancelled;
            message = Some("cancelled".into());
            break;
        }

        before_step(step);
        let result = runner.run_step(step, cancel, opts.stdio);
        after_step(step, &result);
        last_step_id = Some(result.step_id.clone());
        last_exit_code = result.exit_code;
        message = result.message.clone();

        if result.cancelled || cancel.is_cancelled() {
            outcome = RecipeRunOutcome::Cancelled;
            break;
        }
        if !result.started {
            outcome = RecipeRunOutcome::Failed;
            break;
        }
        match result.exit_code {
            Some(code) if code != 0 && !step.continue_on_error => {
                outcome = RecipeRunOutcome::Failed;
                break;
            }
            Some(_) => {}
            // Started process with no exit code: terminated by signal → Cancelled (SEC-2).
            None => {
                outcome = RecipeRunOutcome::Cancelled;
                if message.is_none() {
                    message = Some("process terminated by signal".into());
                }
                break;
            }
        }
    }

    Ok(RecipeExecuteReport {
        outcome,
        last_step_id,
        last_exit_code,
        message,
    })
}

pub fn record_recipe_run_outcome(
    repo: &dyn CommandRecipesRepository,
    recipe_id: &str,
    outcome: RecipeRunOutcome,
    now_unix: i64,
) {
    if let Err(err) = repo.record_run(recipe_id, outcome, now_unix) {
        warn!(recipe_id = %recipe_id, error = %err, "failed to record recipe run");
    }
}

pub fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_secs()).unwrap_or(0))
        .unwrap_or(0)
}

/// Wire Ctrl+C to `cancel` for CLI/TUI recipe runs (multi-thread runtime).
pub fn spawn_ctrl_c_cancel(cancel: CancellationToken) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            cancel.cancel();
        }
    })
}

/// Map a recipe run outcome to a CLI/engine action DTO after execution.
pub fn recipe_outcome_to_action_dto(
    recipe_id: &str,
    report: &RecipeExecuteReport,
) -> luma_protocol::ActionOutcomeDto {
    match report.outcome {
        RecipeRunOutcome::Success => luma_protocol::ActionOutcomeDto::Success {
            message: Some(format!("recipe `{recipe_id}` finished")),
        },
        RecipeRunOutcome::Cancelled => luma_protocol::ActionOutcomeDto::Cancelled,
        RecipeRunOutcome::Failed => {
            let reason = report
                .message
                .clone()
                .or_else(|| {
                    report
                        .last_exit_code
                        .map(|code| format!("recipe `{recipe_id}` exited with code {code}"))
                })
                .unwrap_or_else(|| format!("recipe `{recipe_id}` failed"));
            luma_protocol::ActionOutcomeDto::failed(luma_domain::FailureKind::Unavailable {
                reason,
                retryable: false,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::FakeCommandRunner;
    use luma_domain::RecipeRisk;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn sample_plan(risk: RecipeRisk, continue_on_error: bool) -> RecipeRunPlan {
        RecipeRunPlan {
            recipe_id: "test".into(),
            recipe_title: "Test".into(),
            risk,
            working_dir: PathBuf::from("/tmp"),
            variant_id: "default".into(),
            variant_description: "default".into(),
            steps: vec![ResolvedCommandStep {
                id: "s1".into(),
                label: "step".into(),
                program: "true".into(),
                args: vec![],
                cwd: PathBuf::from("/tmp"),
                root: PathBuf::from("/tmp"),
                continue_on_error,
            }],
        }
    }

    #[test]
    fn confirmation_required_for_non_safe() {
        let runner = FakeCommandRunner::new();
        let plan = sample_plan(RecipeRisk::Confirm, false);
        let err = execute_recipe_plan(
            &plan,
            &runner,
            &CancellationToken::new(),
            RecipeExecuteOptions {
                confirmation: false,
                stdio: RecipeStdioMode::Null,
            },
        )
        .unwrap_err();
        assert!(matches!(
            err,
            RecipeExecuteError::ConfirmationRequired { .. }
        ));
        assert!(runner.calls.lock().unwrap().is_empty());
    }

    #[test]
    fn success_records_zero_exit() {
        let runner = FakeCommandRunner::new();
        let plan = sample_plan(RecipeRisk::Safe, false);
        let report = execute_recipe_plan(
            &plan,
            &runner,
            &CancellationToken::new(),
            RecipeExecuteOptions::default(),
        )
        .unwrap();
        assert_eq!(report.outcome, RecipeRunOutcome::Success);
    }

    #[test]
    fn nonzero_exit_is_failed() {
        let runner = FakeCommandRunner::new();
        runner.push_exit_code(1);
        let plan = sample_plan(RecipeRisk::Safe, false);
        let report = execute_recipe_plan(
            &plan,
            &runner,
            &CancellationToken::new(),
            RecipeExecuteOptions::default(),
        )
        .unwrap();
        assert_eq!(report.outcome, RecipeRunOutcome::Failed);
        assert_eq!(report.last_exit_code, Some(1));
    }

    #[test]
    fn signal_style_none_exit_is_cancelled() {
        let runner = FakeCommandRunner::new();
        runner.push_signal_termination();
        let plan = sample_plan(RecipeRisk::Safe, false);
        let report = execute_recipe_plan(
            &plan,
            &runner,
            &CancellationToken::new(),
            RecipeExecuteOptions::default(),
        )
        .unwrap();
        assert_eq!(report.outcome, RecipeRunOutcome::Cancelled);
    }

    #[test]
    fn pre_cancel_is_cancelled() {
        let runner = FakeCommandRunner::new();
        let plan = sample_plan(RecipeRisk::Safe, false);
        let cancel = CancellationToken::new();
        cancel.cancel();
        let report =
            execute_recipe_plan(&plan, &runner, &cancel, RecipeExecuteOptions::default()).unwrap();
        assert_eq!(report.outcome, RecipeRunOutcome::Cancelled);
        assert!(runner.calls.lock().unwrap().is_empty());
    }

    #[test]
    fn hooks_observe_steps() {
        let runner = FakeCommandRunner::new();
        let plan = sample_plan(RecipeRisk::Safe, false);
        let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen_before = seen.clone();
        let seen_after = seen.clone();
        execute_recipe_plan_with_hooks(
            &plan,
            &runner,
            &CancellationToken::new(),
            RecipeExecuteOptions::default(),
            move |step| {
                seen_before
                    .lock()
                    .unwrap()
                    .push(format!("before:{}", step.id))
            },
            move |step, result| {
                seen_after.lock().unwrap().push(format!(
                    "after:{}:{}",
                    step.id,
                    result.exit_code.unwrap_or(-1)
                ));
            },
        )
        .unwrap();
        let events = seen.lock().unwrap().clone();
        assert_eq!(
            events,
            vec!["before:s1".to_string(), "after:s1:0".to_string()]
        );
    }
}
