use crate::effect::Effect;
use crate::msg::Msg;
use crate::reducer::{command_recipes_query_active, explicit_command_prompt, update};
use crate::render::render;
use crate::terminal::{install_panic_hook, TerminalGuard};
use crate::view_model::{AppState, Route, StatusTone};
use crossterm::event::{self, Event as CEvent, KeyCode, KeyEventKind, KeyModifiers};
use luma_application::run_interactive_terminal;
use luma_application::{
    execute_recipe_plan_with_hooks, now_unix, spawn_ctrl_c_cancel, CommandRunnerPort, EnginePort,
    RecipeExecuteOptions, RecipeStdioMode,
};
use luma_domain::RecipeRunOutcome;
use luma_protocol::Command;
use std::process::ExitStatus;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::warn;

/// Interactive TUI entry. Composition root (`bins/luma`) supplies the engine port.
pub async fn run_tui_with_engine(
    engine: Arc<dyn EnginePort>,
    command_runner: Arc<dyn CommandRunnerPort>,
) -> std::io::Result<()> {
    run_tui_with_options(engine, command_runner, RunTuiOptions::default()).await
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RunTuiOptions {
    /// Seed the editable prompt without submitting or executing it.
    pub initial_query: Option<String>,
}

/// Interactive TUI entry with launch-time prompt options.
pub async fn run_tui_with_options(
    engine: Arc<dyn EnginePort>,
    command_runner: Arc<dyn CommandRunnerPort>,
    options: RunTuiOptions,
) -> std::io::Result<()> {
    install_panic_hook();
    let mut guard = TerminalGuard::enter()?;
    let mut state = AppState::default();
    if let Some(initial_query) = options.initial_query {
        state.prompt = initial_query;
        state.prompt_cursor = state.prompt.chars().count();
    }
    if let Ok((width, height)) = crossterm::terminal::size() {
        state.term_width = width;
        state.term_height = height;
        state.sync_results_viewport();
        state.ensure_prompt_visible(width.saturating_sub(2) as usize);
    }
    state.status.set("Starting…", StatusTone::Progress);
    state.dirty = true;

    guard.terminal_mut().draw(|f| render(f, &state))?;
    state.dirty = false;

    let mut engine_rx = engine.subscribe();
    let mut effect_tasks: JoinSet<()> = JoinSet::new();
    let engine_start = engine.clone();
    effect_tasks.spawn(async move {
        let _ = engine_start.submit(Command::StartSession).await;
    });

    loop {
        // Reap completed submissions continuously; a long-lived TUI should not retain one
        // completed JoinHandle per search or key event until shutdown.
        while let Some(joined) = effect_tasks.try_join_next() {
            if let Err(err) = joined {
                warn!(?err, "TUI effect task ended with error");
            }
        }

        if let Some(plan) = state.pending_recipe_run.take() {
            run_recipe_in_terminal(
                &mut guard,
                command_runner.as_ref(),
                engine.clone(),
                &mut state,
                plan,
                &mut effect_tasks,
            );
            if command_recipes_query_active(&state.prompt) && !state.prompt.trim().is_empty() {
                let effects = update(&mut state, Msg::FlushSearch);
                for effect in effects {
                    dispatch_effect(engine.clone(), effect, &mut effect_tasks);
                }
            }
            if state.should_quit {
                break;
            }
        }

        if state.dirty {
            guard.terminal_mut().draw(|f| render(f, &state))?;
            state.dirty = false;
        }

        let poll_timeout = Duration::from_millis(33);
        let mut msgs: Vec<Msg> = Vec::new();
        let mut broadcast_lagged = false;

        loop {
            match engine_rx.try_recv() {
                Ok(ev) => msgs.push(Msg::Engine(ev)),
                Err(tokio::sync::broadcast::error::TryRecvError::Lagged(n)) => {
                    warn!(skipped = n, "broadcast subscriber lagged");
                    broadcast_lagged = true;
                    continue;
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Empty)
                | Err(tokio::sync::broadcast::error::TryRecvError::Closed) => break,
            }
        }

        if broadcast_lagged {
            msgs.push(Msg::BroadcastLagged);
        }

        if let Some(deadline) = state.search_debounce_deadline {
            if std::time::Instant::now() >= deadline {
                msgs.push(Msg::FlushSearch);
            }
        }

        if let Some(deadline) = state.hub_refresh_deadline {
            if std::time::Instant::now() >= deadline {
                msgs.push(Msg::RefreshHub);
            }
        }

        if event::poll(poll_timeout)? {
            match event::read()? {
                CEvent::Key(key) if key.kind == KeyEventKind::Press => {
                    msgs.push(map_key(key.code, key.modifiers, &state));
                }
                CEvent::Resize(width, height) => msgs.push(Msg::Resize { width, height }),
                CEvent::FocusGained => msgs.push(Msg::FocusGained),
                CEvent::Paste(s) => {
                    msgs.extend(paste_msgs(&state.route, &s));
                }
                _ => {}
            }
        }

        for msg in msgs {
            let effects = update(&mut state, msg);
            for effect in effects {
                if !handle_effect_sync(
                    SyncEffectRuntime {
                        engine: engine.clone(),
                        guard: &mut guard,
                        state: &mut state,
                        tasks: &mut effect_tasks,
                    },
                    effect.clone(),
                ) {
                    dispatch_effect(engine.clone(), effect, &mut effect_tasks);
                }
            }
        }

        if state.should_quit {
            break;
        }
    }

    effect_tasks.abort_all();
    while effect_tasks.join_next().await.is_some() {}
    drop(guard);
    let _ = engine.submit(Command::ShutdownSession).await;
    Ok(())
}

fn run_recipe_in_terminal(
    guard: &mut TerminalGuard,
    runner: &dyn CommandRunnerPort,
    engine: Arc<dyn EnginePort>,
    state: &mut AppState,
    plan: luma_domain::RecipeRunPlan,
    tasks: &mut JoinSet<()>,
) {
    if let Err(err) = guard.suspend() {
        state
            .status
            .set(format!("failed to suspend TUI: {err}"), StatusTone::Error);
        state.dirty = true;
        // Do not keep drawing into a terminal whose raw/alternate-screen transition is
        // uncertain. Dropping the guard immediately gives its cleanup path another chance.
        state.should_quit = true;
        return;
    }
    println!(
        "\n=== Recipe: {} ({}) ===",
        plan.recipe_title, plan.recipe_id
    );
    println!("Risk: {}", plan.risk.as_str());
    println!("Working directory: {}", plan.working_dir.display());
    println!(
        "Variant: {} — {}",
        plan.variant_id, plan.variant_description
    );

    let cancel = CancellationToken::new();
    let cancel_task = spawn_ctrl_c_cancel(cancel.clone());
    // Confirmation already granted by TUI Confirm overlay / safe risk before ExecuteAction.
    let report = execute_recipe_plan_with_hooks(
        &plan,
        runner,
        &cancel,
        RecipeExecuteOptions {
            confirmation: true,
            stdio: RecipeStdioMode::Inherit,
        },
        |step| {
            println!("\n→ {}", step.label);
        },
        |_, result| {
            if result.cancelled {
                println!("cancelled");
            } else if let Some(code) = result.exit_code {
                println!("exit code: {code}");
            } else if result.started {
                println!("exit code: (signal)");
            } else {
                println!(
                    "failed to start: {}",
                    result
                        .message
                        .clone()
                        .unwrap_or_else(|| "unknown error".into())
                );
            }
        },
    );
    cancel_task.abort();

    let outcome = match report {
        Ok(report) => report.outcome,
        Err(_) => RecipeRunOutcome::Failed,
    };

    println!("\n=== Recipe finished ===\n");
    if let Err(err) = guard.resume() {
        state
            .status
            .set(format!("terminal resume failed: {err}"), StatusTone::Error);
        state.should_quit = true;
    } else {
        let tone = match outcome {
            RecipeRunOutcome::Success => StatusTone::Success,
            RecipeRunOutcome::Failed => StatusTone::Error,
            RecipeRunOutcome::Cancelled => StatusTone::Warning,
        };
        state
            .status
            .set(format!("recipe {} finished", plan.recipe_id), tone);
    }
    state.dirty = true;
    let recipe_id = plan.recipe_id.clone();
    let now = now_unix();
    tasks.spawn(async move {
        let _ = engine
            .submit(Command::RecordRecipeRun {
                recipe_id,
                result: outcome,
                now_unix: now,
            })
            .await;
    });
}

/// Terminal paste bypasses `map_key`; ignore it on overlays so Confirm/Settings stay open.
fn paste_msgs(route: &Route, pasted: &str) -> Vec<Msg> {
    if matches!(
        route,
        Route::ConfirmAction
            | Route::ActionPicker
            | Route::Settings
            | Route::Commands
            | Route::QuitConfirm
    ) {
        return Vec::new();
    }
    pasted.chars().map(Msg::KeyChar).collect()
}

fn map_key(code: KeyCode, modifiers: KeyModifiers, state: &AppState) -> Msg {
    use crate::view_model::FocusZone;

    if modifiers.contains(KeyModifiers::CONTROL) {
        return match code {
            KeyCode::Char('c') => Msg::Quit,
            KeyCode::Char('l') => Msg::Redraw,
            KeyCode::Char('k') if matches!(state.route, Route::Search) => Msg::OpenActions,
            KeyCode::Char('/') | KeyCode::Char('_') | KeyCode::Char('\u{1f}')
                if matches!(state.route, Route::Search) =>
            {
                Msg::OpenCommands
            }
            KeyCode::Char('p') => {
                if state.focus == FocusZone::Prompt {
                    Msg::HistoryOlder
                } else {
                    Msg::SelectPrev
                }
            }
            KeyCode::Char('n') => {
                if state.focus == FocusZone::Prompt {
                    Msg::HistoryNewer
                } else {
                    Msg::SelectNext
                }
            }
            KeyCode::Char('u') => Msg::ClearToStart,
            KeyCode::Char('w') => Msg::DeleteWordBack,
            KeyCode::Char('a') => Msg::CursorHome,
            KeyCode::Char('e') => Msg::CursorEnd,
            _ => Msg::Tick,
        };
    }
    match code {
        KeyCode::BackTab if matches!(state.route, Route::Search) => Msg::TogglePreview,
        KeyCode::Tab if matches!(state.route, Route::Search) => Msg::FocusNext,
        KeyCode::Char('\u{1f}') if matches!(state.route, Route::Search) => Msg::OpenCommands,
        KeyCode::Char('?') if matches!(state.route, Route::Search) => Msg::OpenHelp,
        KeyCode::Char(c)
            if state.should_intercept_window_digit() && c.is_ascii_digit() && c != '0' =>
        {
            Msg::PickWindowDigit(c.to_digit(10).unwrap_or(0) as usize)
        }
        KeyCode::Char(c)
            if matches!(state.route, Route::WordbookReview)
                && matches!(c, '1' | '2' | '3' | 'm' | 'M' | 's' | 'S') =>
        {
            let action = match c {
                '1' => "known",
                '2' => "fuzzy",
                '3' => "unknown",
                'm' | 'M' => "mastered",
                _ => "skip",
            };
            Msg::WordbookGrade {
                action_id: action.into(),
            }
        }
        KeyCode::Char(c)
            if matches!(state.route, Route::ActionPicker) && c.is_ascii_digit() && c != '0' =>
        {
            Msg::PickActionDigit(c.to_digit(10).unwrap_or(0) as usize)
        }
        KeyCode::Char(c)
            if matches!(state.route, Route::Search) && state.focus != FocusZone::Prompt =>
        {
            if let Some(item) = state.selected_search_item() {
                if let Some(action_id) =
                    crate::module_shortcuts::list_shortcut_action(item.module_id.as_str(), c)
                {
                    return Msg::RecipeShortcut {
                        action_id: action_id.into(),
                    };
                }
            }
            Msg::KeyChar(c)
        }
        KeyCode::Char(' ') if matches!(state.route, Route::WordbookReview) => Msg::WordbookReveal,
        KeyCode::Char(' ') if matches!(state.route, Route::Settings) => Msg::ToggleSetting,
        KeyCode::Char(c) if matches!(state.route, Route::Search | Route::Help) => Msg::KeyChar(c),
        KeyCode::Char(_) => Msg::Tick,
        KeyCode::Backspace => Msg::Backspace,
        KeyCode::Delete => Msg::DeleteForward,
        KeyCode::Enter => Msg::Submit,
        KeyCode::Left => Msg::CursorLeft,
        KeyCode::Right => Msg::CursorRight,
        KeyCode::Home => Msg::CursorHome,
        KeyCode::End => Msg::CursorEnd,
        KeyCode::Up => Msg::SelectPrev,
        KeyCode::Down => Msg::SelectNext,
        KeyCode::PageUp => Msg::SelectPageUp,
        KeyCode::PageDown => Msg::SelectPageDown,
        KeyCode::Esc => Msg::Cancel,
        _ => Msg::Tick,
    }
}

struct SyncEffectRuntime<'a> {
    engine: Arc<dyn EnginePort>,
    guard: &'a mut TerminalGuard,
    state: &'a mut AppState,
    tasks: &'a mut JoinSet<()>,
}

fn handle_effect_sync(runtime: SyncEffectRuntime<'_>, effect: Effect) -> bool {
    match effect {
        Effect::RunInteractiveTerminal {
            program,
            args,
            record_alias,
            operation_id,
        } => {
            run_interactive_terminal_effect(runtime, program, args, record_alias, operation_id);
            true
        }
        _ => false,
    }
}

fn run_interactive_terminal_effect(
    runtime: SyncEffectRuntime<'_>,
    program: String,
    args: Vec<String>,
    record_alias: Option<String>,
    operation_id: String,
) {
    let SyncEffectRuntime {
        engine,
        guard,
        state,
        tasks,
    } = runtime;
    let suspend_result = guard.suspend();
    let spawn_result = match suspend_result {
        Ok(()) => Some(run_interactive_terminal(&program, &args)),
        Err(err) => {
            state
                .status
                .set(format!("failed to suspend TUI: {err}"), StatusTone::Error);
            state.dirty = true;
            state.should_quit = true;
            None
        }
    };

    let terminal_started = spawn_result.is_some();
    let (status, message, tone) = match spawn_result {
        Some(Ok(status)) => interactive_status(&program, status),
        Some(Err(err)) => (
            None,
            format!("failed to start {program}: {err}"),
            StatusTone::Error,
        ),
        None => (None, state.status.text.clone(), StatusTone::Error),
    };

    if terminal_started {
        if let Err(err) = guard.resume() {
            state
                .status
                .set(format!("failed to restore TUI: {err}"), StatusTone::Error);
            state.should_quit = true;
            state.dirty = true;
        } else {
            state.status.set(message, tone);
            state.dirty = true;
        }
    } else {
        state.status.set(message, tone);
        state.dirty = true;
    }

    if state.active_operation.as_deref() == Some(operation_id.as_str()) {
        state.active_operation = None;
    }

    if let (Some(alias), Some(status)) = (record_alias, status) {
        if status.success() {
            if explicit_command_prompt(&state.prompt)
                .is_some_and(|command| command.starts_with("ssh"))
            {
                state.search_debounce_deadline = Some(std::time::Instant::now());
            }
            let engine_record = engine.clone();
            tasks.spawn(async move {
                let _ = engine_record
                    .submit(Command::SshSessionEnded {
                        alias,
                        exit_code: status.code().unwrap_or(1),
                    })
                    .await;
            });
        }
    }
}

fn interactive_status(
    program: &str,
    status: ExitStatus,
) -> (Option<ExitStatus>, String, StatusTone) {
    if status.success() {
        (
            Some(status),
            format!("{program} exited"),
            StatusTone::Success,
        )
    } else {
        let code = status.code().unwrap_or(1);
        (
            Some(status),
            format!("{program} exited with code {code}"),
            StatusTone::Warning,
        )
    }
}

fn dispatch_effect(engine: Arc<dyn EnginePort>, effect: Effect, tasks: &mut JoinSet<()>) {
    match effect {
        Effect::Search { request_id, query } => {
            tasks.spawn(async move {
                let _ = engine.submit(Command::Search { request_id, query }).await;
            });
        }
        Effect::CancelSearch { request_id } => {
            tasks.spawn(async move {
                let _ = engine.submit(Command::CancelSearch { request_id }).await;
            });
        }
        Effect::LoadHub => {
            tasks.spawn(async move {
                let _ = engine.submit(Command::LoadHub).await;
            });
        }
        Effect::LoadWordbookReview { queue } => {
            tasks.spawn(async move {
                let _ = engine.submit(Command::LoadWordbookReview { queue }).await;
            });
        }
        Effect::RefreshWordbookReviewStats => {
            tasks.spawn(async move {
                let _ = engine.submit(Command::RefreshWordbookReviewStats).await;
            });
        }
        Effect::GetSnapshot => {
            tasks.spawn(async move {
                let _ = engine.submit(Command::GetSnapshot).await;
            });
        }
        Effect::LoadPreview {
            result_id,
            preview_id,
        } => {
            tasks.spawn(async move {
                let _ = engine
                    .submit(Command::LoadPreview {
                        result_id,
                        preview_id,
                    })
                    .await;
            });
        }
        Effect::ListActions { result_id } => {
            tasks.spawn(async move {
                let _ = engine.submit(Command::ListActions { result_id }).await;
            });
        }
        Effect::ExecuteAction {
            operation_id,
            result_id,
            action_id,
            confirmation,
        } => {
            tasks.spawn(async move {
                let _ = engine
                    .submit(Command::ExecuteAction {
                        operation_id,
                        result_id,
                        action_id,
                        confirmation,
                    })
                    .await;
            });
        }
        Effect::CancelOperation { operation_id } => {
            tasks.spawn(async move {
                let _ = engine
                    .submit(Command::CancelOperation { operation_id })
                    .await;
            });
        }
        Effect::RecordRecipeRun {
            recipe_id,
            result,
            now_unix,
        } => {
            tasks.spawn(async move {
                let _ = engine
                    .submit(Command::RecordRecipeRun {
                        recipe_id,
                        result,
                        now_unix,
                    })
                    .await;
            });
        }
        Effect::GetSettings => {
            tasks.spawn(async move {
                let _ = engine.submit(Command::GetSettings).await;
            });
        }
        Effect::UpdateSettings {
            module_id,
            enabled,
            expected_version,
        } => {
            tasks.spawn(async move {
                let _ = engine
                    .submit(Command::UpdateSettings {
                        patch: serde_json::json!({
                            "enabled_modules": { module_id: enabled }
                        }),
                        expected_version,
                    })
                    .await;
            });
        }
        Effect::None => {}
        Effect::RunInteractiveTerminal { .. } => {
            warn!(
                "RunInteractiveTerminal reached async dispatch — should be handled synchronously"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paste_on_confirm_is_ignored() {
        assert!(paste_msgs(&Route::ConfirmAction, "y").is_empty());
        assert!(paste_msgs(&Route::Settings, "toggle").is_empty());
        let search = paste_msgs(&Route::Search, "ab");
        assert_eq!(search.len(), 2);
        assert!(matches!(search[0], Msg::KeyChar('a')));
        assert!(matches!(search[1], Msg::KeyChar('b')));
    }

    #[test]
    fn ctrl_underscore_encoding_opens_commands() {
        let state = AppState::default();
        let msg = map_key(KeyCode::Char('_'), KeyModifiers::CONTROL, &state);
        assert!(matches!(msg, Msg::OpenCommands));
    }

    #[test]
    fn ctrl_slash_control_character_opens_commands() {
        let state = AppState::default();
        let msg = map_key(KeyCode::Char('\u{1f}'), KeyModifiers::empty(), &state);
        assert!(matches!(msg, Msg::OpenCommands));
    }

    #[test]
    fn map_key_digit_routes_to_prompt_when_not_intercepting() {
        let mut state = AppState::default();
        state.prompt = "app ".into();
        state.prompt_cursor = state.prompt_char_len();
        let msg = map_key(KeyCode::Char('3'), KeyModifiers::empty(), &state);
        assert!(matches!(msg, Msg::KeyChar('3')));
    }

    #[test]
    fn map_key_digit_routes_to_window_pick_on_hub() {
        let state = AppState::default();
        let msg = map_key(KeyCode::Char('2'), KeyModifiers::empty(), &state);
        assert!(matches!(msg, Msg::PickWindowDigit(2)));
    }

    #[test]
    fn map_key_action_picker_digit_unchanged() {
        let state = AppState {
            route: Route::ActionPicker,
            ..Default::default()
        };
        let msg = map_key(KeyCode::Char('1'), KeyModifiers::empty(), &state);
        assert!(matches!(msg, Msg::PickActionDigit(1)));
    }

    #[test]
    fn map_key_module_list_shortcut_from_table() {
        use crate::view_model::FocusZone;
        use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ModuleId, ResultId, SearchItem};

        let mut state = AppState {
            route: Route::Search,
            focus: FocusZone::List,
            ..Default::default()
        };
        state.results.items.push(SearchItem {
            id: ResultId::new("recipe:1"),
            module_id: ModuleId::new("luma.command_recipes"),
            title: "Build".into(),
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
        });
        state.results.select_at(0);
        let msg = map_key(KeyCode::Char('r'), KeyModifiers::empty(), &state);
        assert!(matches!(
            msg,
            Msg::RecipeShortcut { action_id } if action_id == "run"
        ));
    }

    #[test]
    fn drain_continues_after_lagged() {
        let (tx, _rx) = tokio::sync::broadcast::channel::<u32>(2);
        let mut lagged_rx = tx.subscribe();
        for i in 0..20 {
            let _ = tx.send(i);
        }
        let mut got = Vec::new();
        loop {
            match lagged_rx.try_recv() {
                Ok(v) => got.push(v),
                Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::TryRecvError::Empty)
                | Err(tokio::sync::broadcast::error::TryRecvError::Closed) => break,
            }
        }
        assert!(!got.is_empty());
    }
}
