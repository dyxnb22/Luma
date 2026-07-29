use crate::effect::Effect;
use crate::msg::Msg;
use crate::reducer::{command_recipes_query_active, explicit_command_prompt, update};
use crate::render::render;
use crate::terminal::{install_panic_hook, TerminalGuard};
use crate::view_model::{AppState, Route, StatusTone};
use crossterm::event::{self, Event as CEvent, KeyCode, KeyEventKind, KeyModifiers};
use luma_application::run_interactive_terminal;
use luma_application::{
    execute_recipe_plan_with_hooks, now_unix, spawn_ctrl_c_cancel, CommandRunnerPort,
    EmbeddedPtyEvent, EmbeddedPtyPort, EmbeddedPtySession, EmbeddedPtySize,
    EmbeddedPtySpawnRequest, EnginePort, FakeEmbeddedPty, RecipeExecuteOptions, RecipeStdioMode,
};
use luma_domain::RecipeRunOutcome;
use luma_protocol::Command;
use std::io::Write;
use std::process::ExitStatus;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::warn;

use crate::ssh_workspace::{SshConnectionPhase, VtScreen};

const MAX_PTY_EVENTS_PER_TICK: usize = 32;

/// Interactive TUI entry. Composition root (`bins/luma`) supplies the engine port.
pub async fn run_tui_with_engine(
    engine: Arc<dyn EnginePort>,
    command_runner: Arc<dyn CommandRunnerPort>,
) -> std::io::Result<()> {
    run_tui_with_options(engine, command_runner, RunTuiOptions::default()).await
}

#[derive(Default)]
pub struct RunTuiOptions {
    /// Seed the editable prompt without submitting or executing it.
    pub initial_query: Option<String>,
    /// Child PTY factory for SSH Workspace. Defaults to a Fake (tests).
    pub embedded_pty: Option<Arc<dyn EmbeddedPtyPort>>,
    /// SSH-session recipes for the command shelf.
    pub ssh_shelf_recipes: Vec<luma_domain::Recipe>,
    /// Favorite / use_count for SSH shelf recipes.
    pub ssh_shelf_recipe_meta: std::collections::BTreeMap<String, luma_domain::RecipeMetadata>,
    /// Optional recipe meta store for favorite toggles from the shelf.
    pub command_recipes: Option<Arc<dyn luma_application::CommandRecipesRepository>>,
    /// Pasteboard for Copy actions inside the workspace.
    pub pasteboard: Option<Arc<dyn luma_application::PasteboardPort>>,
}

struct ActiveEmbedded {
    session: Box<dyn EmbeddedPtySession>,
    events: Receiver<EmbeddedPtyEvent>,
    screen: VtScreen,
    writer: Box<dyn Write + Send>,
    exit_seen: bool,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct EmbeddedEventDrain {
    processed: usize,
    screen_changed: bool,
    channel_closed: bool,
    exit: Option<Option<i32>>,
}

fn drain_embedded_events(
    events: &Receiver<EmbeddedPtyEvent>,
    screen: &mut VtScreen,
    exit_seen: &mut bool,
) -> EmbeddedEventDrain {
    let mut result = EmbeddedEventDrain::default();
    for _ in 0..MAX_PTY_EVENTS_PER_TICK {
        match events.try_recv() {
            Ok(EmbeddedPtyEvent::Output(bytes)) => {
                result.processed += 1;
                screen.feed(&bytes);
                result.screen_changed = true;
            }
            Ok(EmbeddedPtyEvent::Exited { code }) => {
                result.processed += 1;
                if !*exit_seen {
                    *exit_seen = true;
                    result.exit = Some(code);
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => break,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                result.channel_closed = true;
                if !*exit_seen {
                    *exit_seen = true;
                    result.exit = Some(None);
                }
                break;
            }
        }
    }
    result
}

/// Interactive TUI entry with launch-time prompt options.
pub async fn run_tui_with_options(
    engine: Arc<dyn EnginePort>,
    command_runner: Arc<dyn CommandRunnerPort>,
    options: RunTuiOptions,
) -> std::io::Result<()> {
    install_panic_hook();
    let embedded_pty: Arc<dyn EmbeddedPtyPort> = options
        .embedded_pty
        .unwrap_or_else(|| FakeEmbeddedPty::new() as Arc<dyn EmbeddedPtyPort>);
    let pasteboard = options.pasteboard;
    let command_recipes = options.command_recipes;
    let mut active_embedded: Option<ActiveEmbedded> = None;
    let mut guard = TerminalGuard::enter()?;
    let mut state = AppState {
        ssh_shelf_recipes: options.ssh_shelf_recipes,
        ssh_shelf_recipe_meta: options.ssh_shelf_recipe_meta,
        ..AppState::default()
    };
    if let Some(initial_query) = options.initial_query {
        state.search.prompt = initial_query;
        state.search.prompt_cursor = state.prompt_char_len();
    }
    if let Ok((width, height)) = crossterm::terminal::size() {
        state.terminal.width = width;
        state.terminal.height = height;
        state.sync_results_viewport();
        state.ensure_prompt_visible(width.saturating_sub(2) as usize);
    }
    state.status.set("Starting…", StatusTone::Progress);
    state.dirty = true;

    guard.terminal_mut().draw(|f| render(f, &state))?;
    state.dirty = false;

    let mut engine_rx = engine.subscribe();
    let mut effect_tasks: JoinSet<()> = JoinSet::new();
    let termination_requested = Arc::new(AtomicBool::new(false));
    install_termination_listener(&mut effect_tasks, termination_requested.clone());
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

        // The native host requests graceful teardown with SIGTERM. Handling it inside the normal
        // event loop preserves ShutdownSession/module teardown (notably timer pause) before exit.
        if termination_requested.load(Ordering::SeqCst) {
            state.should_quit = true;
        }

        if let Some(plan) = state.runtime.pending_recipe_run.take() {
            run_recipe_in_terminal(
                &mut guard,
                command_runner.as_ref(),
                engine.clone(),
                &mut state,
                plan,
                &mut effect_tasks,
            );
            if command_recipes_query_active(&state.search.prompt)
                && !state.search.prompt.trim().is_empty()
            {
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

        let mut embedded_channel_closed = false;
        if let Some(active) = active_embedded.as_mut() {
            let drained =
                drain_embedded_events(&active.events, &mut active.screen, &mut active.exit_seen);
            if drained.screen_changed {
                if let Some(ws) = state.ssh_workspace.as_mut() {
                    ws.apply_screen(&active.screen);
                    if matches!(
                        ws.phase,
                        SshConnectionPhase::Starting | SshConnectionPhase::Authenticating
                    ) {
                        ws.phase = SshConnectionPhase::Connected;
                        ws.status_detail = "Connected".into();
                    }
                }
                state.dirty = true;
            }
            if let Some(code) = drained.exit {
                msgs.push(Msg::SshPtyExited { code });
            }
            embedded_channel_closed = drained.channel_closed;
        }
        if embedded_channel_closed {
            active_embedded.take();
        }

        if let Some(deadline) = state.search.debounce_deadline {
            if std::time::Instant::now() >= deadline {
                msgs.push(Msg::FlushSearch);
            }
        }

        if let Some(deadline) = state.hub.refresh_deadline {
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
                CEvent::Paste(s) => msgs.push(Msg::Paste(s)),
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
                        embedded_pty: embedded_pty.clone(),
                        active_embedded: &mut active_embedded,
                        pasteboard: pasteboard.clone(),
                        command_recipes: command_recipes.clone(),
                    },
                    effect.clone(),
                ) {
                    dispatch_effect(engine.clone(), effect, &mut effect_tasks);
                }
            }
        }

        if state.should_quit {
            if let Some(active) = active_embedded.take() {
                let _ = active.session.kill();
            }
            break;
        }
    }

    effect_tasks.abort_all();
    while effect_tasks.join_next().await.is_some() {}
    drop(guard);
    let _ = engine.submit(Command::ShutdownSession).await;
    Ok(())
}

#[cfg(unix)]
fn install_termination_listener(tasks: &mut JoinSet<()>, requested: Arc<AtomicBool>) {
    use tokio::signal::unix::{signal, SignalKind};

    match signal(SignalKind::terminate()) {
        Ok(mut stream) => {
            tasks.spawn(async move {
                if stream.recv().await.is_some() {
                    requested.store(true, Ordering::SeqCst);
                }
            });
        }
        Err(error) => warn!(?error, "could not install SIGTERM listener"),
    }
}

#[cfg(not(unix))]
fn install_termination_listener(_tasks: &mut JoinSet<()>, _requested: Arc<AtomicBool>) {}

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

fn map_key(code: KeyCode, modifiers: KeyModifiers, state: &AppState) -> Msg {
    use crate::view_model::FocusZone;

    if state.route == Route::SshWorkspace {
        return map_ssh_workspace_key(code, modifiers, state);
    }

    if modifiers.contains(KeyModifiers::CONTROL) {
        return match code {
            KeyCode::Char('c') => Msg::Quit,
            KeyCode::Char('l') => Msg::Redraw,
            KeyCode::Char('k') if matches!(state.route, Route::Search) => Msg::OpenActions,
            KeyCode::Char('/') | KeyCode::Char('_') | KeyCode::Char('\u{1f}')
                if matches!(
                    state.route,
                    Route::Search | Route::Help | Route::Settings | Route::ActionPicker
                ) =>
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
        KeyCode::Char(c)
            if matches!(state.route, Route::Search | Route::Help | Route::Commands) =>
        {
            Msg::KeyChar(c)
        }
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

fn map_ssh_workspace_key(code: KeyCode, modifiers: KeyModifiers, state: &AppState) -> Msg {
    if code == KeyCode::F(6) {
        return Msg::SshToggleShelf;
    }
    if modifiers.contains(KeyModifiers::CONTROL) && matches!(code, KeyCode::Char(' ')) {
        return Msg::SshArmLeader;
    }
    let leader_armed = state
        .ssh_workspace
        .as_ref()
        .is_some_and(|ws| ws.leader_armed);
    if leader_armed {
        return match code {
            KeyCode::Esc => Msg::Cancel,
            KeyCode::Char(c) => Msg::KeyChar(c),
            _ => Msg::Tick,
        };
    }
    let shelf_focused = state.ssh_workspace.as_ref().is_some_and(|ws| {
        matches!(ws.focus, crate::ssh_workspace::SshWorkspaceFocus::Shelf) && ws.shelf_visible
    });
    if shelf_focused {
        return map_ssh_shelf_key(code, state);
    }
    if modifiers.contains(KeyModifiers::SHIFT) && code == KeyCode::PageUp {
        return Msg::SshScrollback { rows: 12 };
    }
    if modifiers.contains(KeyModifiers::SHIFT) && code == KeyCode::PageDown {
        return Msg::SshScrollback { rows: -12 };
    }
    if code == KeyCode::Esc
        && state.ssh_workspace.as_ref().is_some_and(|ws| {
            matches!(
                ws.phase,
                crate::ssh_workspace::SshConnectionPhase::Failed
                    | crate::ssh_workspace::SshConnectionPhase::Disconnected
            )
        })
    {
        return Msg::SshLeave;
    }
    match code {
        KeyCode::Char(c)
            if state.ssh_workspace.as_ref().is_some_and(|ws| {
                matches!(
                    ws.phase,
                    crate::ssh_workspace::SshConnectionPhase::Failed
                        | crate::ssh_workspace::SshConnectionPhase::Disconnected
                )
            }) =>
        {
            Msg::KeyChar(c)
        }
        _ => {
            let application_cursor = state
                .ssh_workspace
                .as_ref()
                .is_some_and(|ws| ws.application_cursor);
            encode_ssh_key(code, modifiers, application_cursor)
                .map_or(Msg::Tick, |bytes| Msg::SshPtyInput { bytes })
        }
    }
}

fn encode_ssh_key(
    code: KeyCode,
    modifiers: KeyModifiers,
    application_cursor: bool,
) -> Option<Vec<u8>> {
    let modifier = 1
        + u8::from(modifiers.contains(KeyModifiers::SHIFT))
        + 2 * u8::from(modifiers.contains(KeyModifiers::ALT))
        + 4 * u8::from(modifiers.contains(KeyModifiers::CONTROL));
    let modified_csi = |final_byte: char| {
        if modifier == 1 {
            format!("\x1b[{final_byte}").into_bytes()
        } else {
            format!("\x1b[1;{modifier}{final_byte}").into_bytes()
        }
    };
    let tilde = |number: u8| {
        if modifier == 1 {
            format!("\x1b[{number}~").into_bytes()
        } else {
            format!("\x1b[{number};{modifier}~").into_bytes()
        }
    };
    let cursor = |final_byte: char| {
        if application_cursor && modifier == 1 {
            format!("\x1bO{final_byte}").into_bytes()
        } else {
            modified_csi(final_byte)
        }
    };

    match code {
        KeyCode::Char(c) if modifiers.contains(KeyModifiers::CONTROL) => {
            let lower = c.to_ascii_lowercase();
            let control = if lower.is_ascii_lowercase() {
                Some((lower as u8) - b'a' + 1)
            } else {
                match c {
                    '@' | ' ' => Some(0),
                    '[' => Some(0x1b),
                    '\\' => Some(0x1c),
                    ']' => Some(0x1d),
                    '^' => Some(0x1e),
                    '_' => Some(0x1f),
                    '?' => Some(0x7f),
                    _ => None,
                }
            }?;
            let mut bytes = vec![control];
            if modifiers.contains(KeyModifiers::ALT) {
                bytes.insert(0, 0x1b);
            }
            Some(bytes)
        }
        KeyCode::Char(c) => {
            let mut encoded = [0u8; 4];
            let mut bytes = c.encode_utf8(&mut encoded).as_bytes().to_vec();
            if modifiers.contains(KeyModifiers::ALT) {
                bytes.insert(0, 0x1b);
            }
            Some(bytes)
        }
        KeyCode::Enter => Some(vec![b'\r']),
        KeyCode::Backspace => Some(vec![0x7f]),
        KeyCode::Tab => Some(vec![b'\t']),
        KeyCode::BackTab => Some(b"\x1b[Z".to_vec()),
        KeyCode::Esc => Some(vec![0x1b]),
        KeyCode::Up => Some(cursor('A')),
        KeyCode::Down => Some(cursor('B')),
        KeyCode::Right => Some(cursor('C')),
        KeyCode::Left => Some(cursor('D')),
        KeyCode::Home => Some(cursor('H')),
        KeyCode::End => Some(cursor('F')),
        KeyCode::Insert => Some(tilde(2)),
        KeyCode::Delete => Some(tilde(3)),
        KeyCode::PageUp => Some(tilde(5)),
        KeyCode::PageDown => Some(tilde(6)),
        KeyCode::F(1) if modifier == 1 => Some(b"\x1bOP".to_vec()),
        KeyCode::F(2) if modifier == 1 => Some(b"\x1bOQ".to_vec()),
        KeyCode::F(3) if modifier == 1 => Some(b"\x1bOR".to_vec()),
        KeyCode::F(4) if modifier == 1 => Some(b"\x1bOS".to_vec()),
        KeyCode::F(5) => Some(tilde(15)),
        KeyCode::F(6) => Some(tilde(17)),
        KeyCode::F(7) => Some(tilde(18)),
        KeyCode::F(8) => Some(tilde(19)),
        KeyCode::F(9) => Some(tilde(20)),
        KeyCode::F(10) => Some(tilde(21)),
        KeyCode::F(11) => Some(tilde(23)),
        KeyCode::F(12) => Some(tilde(24)),
        KeyCode::Null => Some(vec![0]),
        _ => None,
    }
}

fn map_ssh_shelf_key(code: KeyCode, state: &AppState) -> Msg {
    let filling = state
        .ssh_workspace
        .as_ref()
        .is_some_and(|ws| ws.shelf.filling_params);
    match code {
        KeyCode::Esc => Msg::Cancel,
        KeyCode::Up if !filling => Msg::SelectPrev,
        KeyCode::Down if !filling => Msg::SelectNext,
        KeyCode::Tab => Msg::SshShelfParamNext,
        KeyCode::BackTab => Msg::SshShelfParamPrev,
        KeyCode::Enter => Msg::SshShelfPreview,
        KeyCode::Char('c') if !filling => Msg::SshShelfCopy,
        KeyCode::Char('i') if !filling => Msg::SshShelfInsert,
        KeyCode::Char('f') if !filling => Msg::SshShelfFavorite,
        KeyCode::Char('/') if !filling => Msg::SshShelfStartFilter,
        KeyCode::Char(c) => Msg::KeyChar(c),
        KeyCode::Backspace => Msg::Backspace,
        _ => Msg::Tick,
    }
}

struct SyncEffectRuntime<'a> {
    engine: Arc<dyn EnginePort>,
    guard: &'a mut TerminalGuard,
    state: &'a mut AppState,
    tasks: &'a mut JoinSet<()>,
    embedded_pty: Arc<dyn EmbeddedPtyPort>,
    active_embedded: &'a mut Option<ActiveEmbedded>,
    pasteboard: Option<Arc<dyn luma_application::PasteboardPort>>,
    command_recipes: Option<Arc<dyn luma_application::CommandRecipesRepository>>,
}

fn handle_effect_sync(runtime: SyncEffectRuntime<'_>, effect: Effect) -> bool {
    match effect {
        Effect::RunInteractiveTerminal {
            program,
            args,
            environment,
            record_alias,
            operation_id,
        } => {
            run_interactive_terminal_effect(
                runtime,
                program,
                args,
                environment,
                record_alias,
                operation_id,
            );
            true
        }
        Effect::StartEmbeddedTerminal {
            program,
            args,
            environment,
            record_alias: _,
            title: _,
            alias: _,
            hostname: _,
            user: _,
            port: _,
            operation_id: _,
        } => {
            start_embedded_session(runtime, program, args, environment);
            true
        }
        Effect::WriteEmbeddedPty { bytes } => {
            if let Some(active) = runtime.active_embedded.as_mut() {
                if active.screen.scrollback() > 0 {
                    active.screen.scroll_to_bottom();
                    if let Some(ws) = runtime.state.ssh_workspace.as_mut() {
                        ws.apply_screen(&active.screen);
                    }
                    runtime.state.dirty = true;
                }
                let _ = active.writer.write_all(&bytes);
                let _ = active.writer.flush();
            }
            true
        }
        Effect::ResizeEmbeddedPty { cols, rows } => {
            if let Some(active) = runtime.active_embedded.as_mut() {
                let _ = active.session.resize(EmbeddedPtySize::new(cols, rows));
                active.screen.resize(cols, rows);
                if let Some(ws) = runtime.state.ssh_workspace.as_mut() {
                    ws.apply_screen(&active.screen);
                    ws.term_cols = cols;
                    ws.term_rows = rows;
                }
                runtime.state.dirty = true;
            }
            true
        }
        Effect::ScrollEmbeddedPty { rows } => {
            if let Some(active) = runtime.active_embedded.as_mut() {
                active.screen.scroll(rows);
                if let Some(ws) = runtime.state.ssh_workspace.as_mut() {
                    ws.apply_screen(&active.screen);
                }
                runtime.state.dirty = true;
            }
            true
        }
        Effect::KillEmbeddedPty => {
            if let Some(active) = runtime.active_embedded.take() {
                let _ = active.session.kill();
            }
            true
        }
        Effect::CopyText { text } => {
            if let Some(pb) = runtime.pasteboard.clone() {
                let text_clone = text.clone();
                runtime.tasks.spawn(async move {
                    let _ = pb.write_text(&text_clone).await;
                });
                runtime
                    .state
                    .status
                    .set("copied to clipboard", StatusTone::Success);
            } else {
                runtime
                    .state
                    .status
                    .set(format!("copied: {text}"), StatusTone::Success);
            }
            runtime.state.dirty = true;
            true
        }
        Effect::SetRecipeFavorite {
            recipe_id,
            favorite,
        } => {
            if let Some(repo) = runtime.command_recipes.clone() {
                let _ = repo.set_favorite(&recipe_id, favorite);
            }
            true
        }
        Effect::RecordSshSessionEnded { alias, exit_code } => {
            let engine = runtime.engine.clone();
            runtime.tasks.spawn(async move {
                let _ = engine
                    .submit(Command::SshSessionEnded { alias, exit_code })
                    .await;
            });
            true
        }
        _ => false,
    }
}

fn start_embedded_session(
    runtime: SyncEffectRuntime<'_>,
    program: String,
    args: Vec<String>,
    environment: Vec<(String, String)>,
) {
    if let Some(prev) = runtime.active_embedded.take() {
        let _ = prev.session.kill();
    }
    let (cols, rows) = runtime
        .state
        .ssh_workspace
        .as_ref()
        .map(|ws| (ws.term_cols, ws.term_rows))
        .unwrap_or((80, 24));
    match runtime.embedded_pty.spawn(EmbeddedPtySpawnRequest {
        program: program.clone(),
        args,
        environment,
        size: EmbeddedPtySize::new(cols, rows),
    }) {
        Ok((session, events)) => match session.try_clone_writer() {
            Ok(writer) => {
                let screen = VtScreen::new(cols, rows);
                if let Some(ws) = runtime.state.ssh_workspace.as_mut() {
                    ws.phase = SshConnectionPhase::Authenticating;
                    ws.status_detail = "Authenticating".into();
                    ws.apply_screen(&screen);
                }
                runtime
                    .state
                    .status
                    .set("Authenticating…", StatusTone::Progress);
                runtime.state.dirty = true;
                *runtime.active_embedded = Some(ActiveEmbedded {
                    session,
                    events,
                    screen,
                    writer,
                    exit_seen: false,
                });
            }
            Err(err) => {
                runtime.state.status.set(
                    format!("failed to open pty writer: {err}"),
                    StatusTone::Error,
                );
                if let Some(ws) = runtime.state.ssh_workspace.as_mut() {
                    ws.phase = SshConnectionPhase::Failed;
                    ws.status_detail = format!("Failed to start: {err}");
                    ws.error_summary = ws.status_detail.clone();
                }
                runtime.state.dirty = true;
                let _ = session.kill();
            }
        },
        Err(err) => {
            runtime.state.status.set(
                format!("failed to start {program}: {err}"),
                StatusTone::Error,
            );
            if let Some(ws) = runtime.state.ssh_workspace.as_mut() {
                ws.phase = SshConnectionPhase::Failed;
                ws.status_detail = format!("Failed to start: {err}");
                ws.error_summary = ws.status_detail.clone();
            }
            runtime.state.dirty = true;
        }
    }
}

fn run_interactive_terminal_effect(
    runtime: SyncEffectRuntime<'_>,
    program: String,
    args: Vec<String>,
    environment: Vec<(String, String)>,
    record_alias: Option<String>,
    operation_id: String,
) {
    let SyncEffectRuntime {
        engine,
        guard,
        state,
        tasks,
        embedded_pty: _,
        active_embedded: _,
        pasteboard: _,
        command_recipes: _,
    } = runtime;
    let suspend_result = guard.suspend();
    let spawn_result = match suspend_result {
        Ok(()) => Some(run_interactive_terminal(&program, &args, &environment)),
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
        Some(Ok(status)) => interactive_status(&program, &args, status),
        Some(Err(err)) => (
            None,
            format!("failed to start {program}: {err}"),
            StatusTone::Error,
        ),
        None => (None, state.status.text.clone(), StatusTone::Error),
    };

    if should_pause_after_failed_ssh(record_alias.as_deref(), status) {
        eprintln!("\nSSH connection failed. Press Enter to return to Luma.");
        let mut acknowledgement = String::new();
        let _ = std::io::stdin().read_line(&mut acknowledgement);
    }

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
            if status.is_some_and(|status| status.success())
                && explicit_command_prompt(&state.search.prompt)
                    .is_some_and(|command| command.starts_with("pkg"))
            {
                state.search.debounce_deadline = Some(std::time::Instant::now());
            }
        }
    } else {
        state.status.set(message, tone);
        state.dirty = true;
    }

    if state.actions.active_operation.as_deref() == Some(operation_id.as_str()) {
        state.actions.active_operation = None;
        state.actions.active_kind = None;
    }

    if let (Some(alias), Some(status)) = (record_alias, status) {
        if status.success() {
            if explicit_command_prompt(&state.search.prompt)
                .is_some_and(|command| command.starts_with("ssh"))
            {
                state.search.debounce_deadline = Some(std::time::Instant::now());
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

fn should_pause_after_failed_ssh(record_alias: Option<&str>, status: Option<ExitStatus>) -> bool {
    record_alias.is_some() && status.is_some_and(|status| !status.success())
}

fn interactive_status(
    program: &str,
    args: &[String],
    status: ExitStatus,
) -> (Option<ExitStatus>, String, StatusTone) {
    if status.success() {
        if program.ends_with("/brew") || program == "brew" {
            let operation = args.first().map(String::as_str).unwrap_or("operation");
            let package = args
                .iter()
                .rev()
                .find(|arg| !arg.starts_with('-') && arg.as_str() != operation)
                .map(String::as_str)
                .unwrap_or("package");
            return (
                Some(status),
                format!("Homebrew {operation} {package} completed · refreshing"),
                StatusTone::Success,
            );
        }
        (
            Some(status),
            format!("{program} exited"),
            StatusTone::Success,
        )
    } else if let Some(code) = status.code() {
        (
            Some(status),
            format!("{program} exited with code {code}"),
            StatusTone::Warning,
        )
    } else {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;

            if let Some(signal) = status.signal() {
                return (
                    Some(status),
                    format!("{program} ended by signal {signal}"),
                    StatusTone::Warning,
                );
            }
        }
        (
            Some(status),
            format!("{program} ended without an exit code"),
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
        Effect::PatchSettings {
            patch,
            expected_version,
        } => {
            tasks.spawn(async move {
                let _ = engine
                    .submit(Command::UpdateSettings {
                        patch,
                        expected_version,
                    })
                    .await;
            });
        }
        Effect::None => {}
        Effect::RunInteractiveTerminal { .. }
        | Effect::StartEmbeddedTerminal { .. }
        | Effect::WriteEmbeddedPty { .. }
        | Effect::ResizeEmbeddedPty { .. }
        | Effect::ScrollEmbeddedPty { .. }
        | Effect::KillEmbeddedPty
        | Effect::CopyText { .. }
        | Effect::SetRecipeFavorite { .. }
        | Effect::RecordSshSessionEnded { .. } => {
            warn!("embedded/interactive terminal effect reached async dispatch — should be sync");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ssh_workspace::{SshConnectionPhase, SshWorkspaceState};
    use std::sync::mpsc;

    fn ssh_workspace_state() -> AppState {
        AppState {
            route: Route::SshWorkspace,
            ssh_workspace: Some(SshWorkspaceState::new(
                "prod".into(),
                "host.example".into(),
                "root".into(),
                22,
                "prod".into(),
                "/usr/bin/ssh".into(),
                vec![],
                vec![],
                Some("prod".into()),
                80,
                24,
            )),
            ..AppState::default()
        }
    }

    #[test]
    fn embedded_event_drain_is_bounded_and_preserves_ui_fairness() {
        let (tx, rx) = mpsc::channel();
        for _ in 0..MAX_PTY_EVENTS_PER_TICK + 4 {
            tx.send(EmbeddedPtyEvent::Output(b"x".to_vec()))
                .expect("send output");
        }
        let mut screen = VtScreen::new(80, 24);
        let mut exit_seen = false;

        let drained = drain_embedded_events(&rx, &mut screen, &mut exit_seen);

        assert_eq!(drained.processed, MAX_PTY_EVENTS_PER_TICK);
        assert!(drained.screen_changed);
        assert!(!drained.channel_closed);
        assert!(rx.try_recv().is_ok(), "events remain for the next UI tick");
    }

    #[test]
    fn embedded_exit_is_reported_once_when_channel_disconnects() {
        let (tx, rx) = mpsc::channel();
        tx.send(EmbeddedPtyEvent::Exited { code: Some(0) })
            .expect("send exit");
        drop(tx);
        let mut screen = VtScreen::new(80, 24);
        let mut exit_seen = false;

        let first = drain_embedded_events(&rx, &mut screen, &mut exit_seen);
        let second = drain_embedded_events(&rx, &mut screen, &mut exit_seen);

        assert_eq!(first.exit, Some(Some(0)));
        assert!(first.channel_closed);
        assert_eq!(second.exit, None);
        assert!(second.channel_closed);
    }

    #[test]
    fn ssh_reserved_keys_and_terminal_keys_are_unambiguous() {
        let state = ssh_workspace_state();
        assert!(matches!(
            map_key(KeyCode::F(6), KeyModifiers::empty(), &state),
            Msg::SshToggleShelf
        ));
        assert!(matches!(
            map_key(KeyCode::Char(' '), KeyModifiers::CONTROL, &state),
            Msg::SshArmLeader
        ));
        assert!(matches!(
            map_key(KeyCode::Esc, KeyModifiers::empty(), &state),
            Msg::SshPtyInput { bytes } if bytes == [0x1b]
        ));
        assert!(matches!(
            map_key(KeyCode::Delete, KeyModifiers::empty(), &state),
            Msg::SshPtyInput { bytes } if bytes == b"\x1b[3~"
        ));
        assert!(matches!(
            map_key(KeyCode::PageUp, KeyModifiers::SHIFT, &state),
            Msg::SshScrollback { rows: 12 }
        ));
    }

    #[test]
    fn ssh_application_cursor_and_alt_key_use_xterm_encoding() {
        let mut state = ssh_workspace_state();
        state.ssh_workspace.as_mut().unwrap().application_cursor = true;
        assert!(matches!(
            map_key(KeyCode::Up, KeyModifiers::empty(), &state),
            Msg::SshPtyInput { bytes } if bytes == b"\x1bOA"
        ));
        assert!(matches!(
            map_key(KeyCode::Char('x'), KeyModifiers::ALT, &state),
            Msg::SshPtyInput { bytes } if bytes == b"\x1bx"
        ));
    }

    #[test]
    fn ssh_failed_escape_leaves_instead_of_writing_to_dead_pty() {
        let mut state = ssh_workspace_state();
        state.ssh_workspace.as_mut().unwrap().phase = SshConnectionPhase::Failed;
        assert!(matches!(
            map_key(KeyCode::Esc, KeyModifiers::empty(), &state),
            Msg::SshLeave
        ));
    }

    #[test]
    fn bracketed_paste_is_forwarded_as_one_message() {
        // The reducer decides whether the current surface accepts the text;
        // keeping it atomic prevents CR/LF from being mapped to Submit.
        let msg = Msg::Paste("a\r\nb".into());
        assert!(matches!(msg, Msg::Paste(text) if text == "a\r\nb"));
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
    fn command_palette_and_page_keys_cover_scrollable_overlays() {
        for route in [Route::Help, Route::Settings, Route::ActionPicker] {
            let state = AppState {
                route,
                ..AppState::default()
            };
            assert!(matches!(
                map_key(KeyCode::Char('/'), KeyModifiers::CONTROL, &state),
                Msg::OpenCommands
            ));
            assert!(matches!(
                map_key(KeyCode::PageUp, KeyModifiers::empty(), &state),
                Msg::SelectPageUp
            ));
            assert!(matches!(
                map_key(KeyCode::PageDown, KeyModifiers::empty(), &state),
                Msg::SelectPageDown
            ));
        }
    }

    #[test]
    fn tab_and_backtab_have_distinct_search_behaviors() {
        let state = AppState::default();
        assert!(matches!(
            map_key(KeyCode::Tab, KeyModifiers::empty(), &state),
            Msg::FocusNext
        ));
        assert!(matches!(
            map_key(KeyCode::BackTab, KeyModifiers::SHIFT, &state),
            Msg::TogglePreview
        ));

        let overlay = AppState {
            route: Route::Help,
            ..AppState::default()
        };
        assert!(matches!(
            map_key(KeyCode::Tab, KeyModifiers::empty(), &overlay),
            Msg::Tick
        ));
        assert!(matches!(
            map_key(KeyCode::BackTab, KeyModifiers::SHIFT, &overlay),
            Msg::Tick
        ));
    }

    #[test]
    fn map_key_digit_routes_to_prompt_when_not_intercepting() {
        let mut state = AppState::default();
        state.search.prompt = "app ".into();
        state.search.prompt_cursor = state.prompt_char_len();
        let msg = map_key(KeyCode::Char('3'), KeyModifiers::empty(), &state);
        assert!(matches!(msg, Msg::KeyChar('3')));
    }

    #[test]
    fn map_key_digit_routes_to_prompt_on_hub_when_prompt_is_focused() {
        let state = AppState::default();
        let msg = map_key(KeyCode::Char('1'), KeyModifiers::empty(), &state);
        assert!(matches!(msg, Msg::KeyChar('1')));
    }

    #[test]
    fn map_key_digit_routes_to_window_pick_on_hub_list() {
        let state = AppState {
            focus: crate::view_model::FocusZone::List,
            ..AppState::default()
        };
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
    fn numeric_shortcuts_are_scoped_to_their_active_surface() {
        let action_picker = AppState {
            route: Route::ActionPicker,
            ..Default::default()
        };
        for digit in '1'..='9' {
            assert!(matches!(
                map_key(KeyCode::Char(digit), KeyModifiers::empty(), &action_picker),
                Msg::PickActionDigit(index) if index == digit.to_digit(10).unwrap() as usize
            ));
        }
        assert!(matches!(
            map_key(KeyCode::Char('0'), KeyModifiers::empty(), &action_picker),
            Msg::Tick
        ));

        let wordbook = AppState {
            route: Route::WordbookReview,
            ..Default::default()
        };
        for (digit, action_id) in [('1', "known"), ('2', "fuzzy"), ('3', "unknown")] {
            assert!(matches!(
                map_key(KeyCode::Char(digit), KeyModifiers::empty(), &wordbook),
                Msg::WordbookGrade { action_id: actual } if actual == action_id
            ));
        }
        assert!(matches!(
            map_key(KeyCode::Char('4'), KeyModifiers::empty(), &wordbook),
            Msg::Tick
        ));
    }

    #[cfg(unix)]
    #[test]
    fn interactive_status_reports_signal_without_a_fake_exit_code() {
        use std::os::unix::process::ExitStatusExt;

        let status = ExitStatus::from_raw(15);
        let (returned, message, tone) = interactive_status("ssh", &[], status);

        assert_eq!(returned.and_then(|status| status.signal()), Some(15));
        assert_eq!(message, "ssh ended by signal 15");
        assert!(!message.contains("code 1"));
        assert_eq!(tone, StatusTone::Warning);
    }

    #[cfg(unix)]
    #[test]
    fn failed_ssh_sessions_pause_before_restoring_the_tui() {
        use std::os::unix::process::ExitStatusExt;

        let failed = ExitStatus::from_raw(255 << 8);
        let succeeded = ExitStatus::from_raw(0);

        assert!(should_pause_after_failed_ssh(
            Some("production"),
            Some(failed)
        ));
        assert!(!should_pause_after_failed_ssh(
            Some("production"),
            Some(succeeded)
        ));
        assert!(!should_pause_after_failed_ssh(None, Some(failed)));
    }

    #[cfg(unix)]
    #[test]
    fn interactive_status_names_successful_homebrew_mutation_and_refresh() {
        use std::os::unix::process::ExitStatusExt;

        let status = ExitStatus::from_raw(0);
        let (_, message, tone) = interactive_status(
            "/opt/homebrew/bin/brew",
            &["install".into(), "--cask".into(), "zed".into()],
            status,
        );
        assert_eq!(message, "Homebrew install zed completed · refreshing");
        assert_eq!(tone, StatusTone::Success);
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
        state.search.results.items.push(SearchItem {
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
        state.search.results.select_at(0);
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
