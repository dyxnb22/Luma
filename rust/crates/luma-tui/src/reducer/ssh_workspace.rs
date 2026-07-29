//! SSH Workspace reducer helpers (pure).

use crate::effect::Effect;
use crate::ssh_workspace::{SshConnectionPhase, SshWorkspaceFocus, SshWorkspaceState};
use crate::view_model::{AppState, FocusZone, Route, StatusTone};

#[allow(clippy::too_many_arguments)]
pub(super) fn open_embedded_workspace(
    state: &mut AppState,
    program: String,
    args: Vec<String>,
    environment: Vec<(String, String)>,
    record_alias: Option<String>,
    title: String,
    alias: String,
    hostname: String,
    user: String,
    port: u16,
) -> Vec<Effect> {
    let width = state.terminal.width.max(1);
    let height = state.terminal.height.max(4);
    // Shelf starts hidden — do not reserve side width until the user opens it.
    let term_cols = width.saturating_sub(2).max(20);
    let term_rows = height.saturating_sub(4).max(3);
    state.ssh_workspace = Some(
        SshWorkspaceState::new(
            alias,
            hostname,
            user,
            port,
            title,
            program.clone(),
            args.clone(),
            environment.clone(),
            record_alias.clone(),
            term_cols,
            term_rows,
        )
        .with_shelf_recipes_and_meta(&state.ssh_shelf_recipes, &state.ssh_shelf_recipe_meta),
    );
    state.route = Route::SshWorkspace;
    state.focus = FocusZone::Terminal;
    state.status.set("Connecting…", StatusTone::Progress);
    vec![Effect::StartEmbeddedTerminal {
        program,
        args,
        environment,
        record_alias,
        title: state
            .ssh_workspace
            .as_ref()
            .map(|s| s.title.clone())
            .unwrap_or_default(),
        alias: state
            .ssh_workspace
            .as_ref()
            .map(|s| s.alias.clone())
            .unwrap_or_default(),
        hostname: state
            .ssh_workspace
            .as_ref()
            .map(|s| s.hostname.clone())
            .unwrap_or_default(),
        user: state
            .ssh_workspace
            .as_ref()
            .map(|s| s.user.clone())
            .unwrap_or_default(),
        port: state.ssh_workspace.as_ref().map(|s| s.port).unwrap_or(22),
        operation_id: "embedded-ssh".into(),
    }]
}

pub(super) fn on_pty_exited(state: &mut AppState, code: Option<i32>) -> Vec<Effect> {
    let Some(ws) = state.ssh_workspace.as_mut() else {
        return vec![Effect::None];
    };
    ws.exit_code = code;
    let alias = ws.record_alias.clone().unwrap_or_else(|| ws.alias.clone());
    let mut effects = Vec::new();
    if code == Some(0) {
        ws.phase = SshConnectionPhase::Disconnected;
        ws.status_detail = "Exited".into();
        effects.push(Effect::RecordSshSessionEnded {
            alias,
            exit_code: 0,
        });
    } else if let Some(code) = code {
        ws.phase = SshConnectionPhase::Failed;
        ws.status_detail = format!("Exited with code {code}");
        ws.error_summary = ws.status_detail.clone();
    } else {
        ws.phase = SshConnectionPhase::Disconnected;
        ws.status_detail = "Connection lost".into();
        ws.error_summary = ws.status_detail.clone();
    }
    state
        .status
        .set(ws.status_detail.clone(), StatusTone::Warning);
    if effects.is_empty() {
        vec![Effect::None]
    } else {
        effects
    }
}

pub(super) fn leave_workspace(state: &mut AppState) -> Vec<Effect> {
    state.ssh_workspace = None;
    state.route = Route::Search;
    state.focus = FocusZone::Prompt;
    state.search.prompt = "/ssh ".into();
    state.search.prompt_cursor = state.prompt_char_len();
    state.status.set("SSH hosts", StatusTone::Neutral);
    vec![
        Effect::KillEmbeddedPty,
        Effect::Search {
            request_id: uuid_like(),
            query: "/ssh ".into(),
        },
    ]
}

pub(super) fn disconnect(state: &mut AppState) -> Vec<Effect> {
    if let Some(ws) = state.ssh_workspace.as_mut() {
        ws.disconnect_confirm = false;
        ws.phase = SshConnectionPhase::Disconnected;
        ws.status_detail = "Disconnected".into();
        ws.error_summary.clear();
    }
    state.status.set("Disconnected", StatusTone::Warning);
    vec![Effect::KillEmbeddedPty]
}

pub(super) fn reconnect(state: &mut AppState) -> Vec<Effect> {
    let Some(ws) = state.ssh_workspace.as_ref() else {
        return vec![Effect::None];
    };
    let program = ws.program.clone();
    let args = ws.args.clone();
    let environment = ws.environment.clone();
    let record_alias = ws.record_alias.clone();
    let title = ws.title.clone();
    let alias = ws.alias.clone();
    let hostname = ws.hostname.clone();
    let user = ws.user.clone();
    let port = ws.port;
    if let Some(ws) = state.ssh_workspace.as_mut() {
        ws.phase = SshConnectionPhase::Starting;
        ws.status_detail = "Reconnecting".into();
        ws.exit_code = None;
        ws.error_summary.clear();
        ws.disconnect_confirm = false;
        ws.leader_armed = false;
        ws.shelf_visible = false;
        ws.focus = SshWorkspaceFocus::Terminal;
    }
    state.focus = FocusZone::Terminal;
    state.status.set("Reconnecting…", StatusTone::Progress);
    vec![
        Effect::KillEmbeddedPty,
        Effect::StartEmbeddedTerminal {
            program,
            args,
            environment,
            record_alias,
            title,
            alias,
            hostname,
            user,
            port,
            operation_id: "embedded-ssh-reconnect".into(),
        },
    ]
}

pub(super) fn compat_reconnect(state: &mut AppState) -> Vec<Effect> {
    let Some(ws) = state.ssh_workspace.as_ref() else {
        return vec![Effect::None];
    };
    let program = ws.program.clone();
    let args = ws.args.clone();
    let environment = ws.environment.clone();
    let record_alias = ws.record_alias.clone();
    state.ssh_workspace = None;
    state.route = Route::Search;
    state.focus = FocusZone::Prompt;
    vec![
        Effect::KillEmbeddedPty,
        Effect::RunInteractiveTerminal {
            program,
            args,
            environment,
            record_alias,
            operation_id: "ssh-compat".into(),
        },
    ]
}

pub(super) fn toggle_shelf(state: &mut AppState) -> Vec<Effect> {
    let Some(ws) = state.ssh_workspace.as_mut() else {
        return vec![Effect::None];
    };
    ws.leader_armed = false;
    ws.disconnect_confirm = false;
    if ws.shelf_visible && matches!(ws.focus, SshWorkspaceFocus::Shelf) {
        ws.shelf_visible = false;
        ws.focus = SshWorkspaceFocus::Terminal;
        state.focus = FocusZone::Terminal;
    } else {
        ws.shelf_visible = true;
        ws.fullscreen_chrome = false;
        ws.focus = SshWorkspaceFocus::Shelf;
        state.focus = FocusZone::CommandShelf;
    }
    let (cols, rows) = terminal_geometry(state);
    if let Some(ws) = state.ssh_workspace.as_mut() {
        ws.term_cols = cols;
        ws.term_rows = rows;
    }
    vec![Effect::ResizeEmbeddedPty { cols, rows }]
}

pub(super) fn arm_leader(state: &mut AppState) -> Vec<Effect> {
    let Some(ws) = state.ssh_workspace.as_mut() else {
        return vec![Effect::None];
    };
    ws.leader_armed = true;
    ws.shelf_visible = false;
    ws.focus = SshWorkspaceFocus::Leader;
    state.focus = FocusZone::Terminal;
    let (cols, rows) = terminal_geometry(state);
    if let Some(ws) = state.ssh_workspace.as_mut() {
        ws.term_cols = cols;
        ws.term_rows = rows;
    }
    vec![Effect::ResizeEmbeddedPty { cols, rows }]
}

/// Close shelf chrome and return focus to the terminal (Esc from shelf).
pub(super) fn shelf_back_to_terminal(state: &mut AppState) -> Vec<Effect> {
    let Some(ws) = state.ssh_workspace.as_mut() else {
        return vec![Effect::None];
    };
    ws.shelf_visible = false;
    ws.leader_armed = false;
    ws.disconnect_confirm = false;
    ws.focus = SshWorkspaceFocus::Terminal;
    state.focus = FocusZone::Terminal;
    let (cols, rows) = terminal_geometry(state);
    if let Some(ws) = state.ssh_workspace.as_mut() {
        ws.term_cols = cols;
        ws.term_rows = rows;
    }
    vec![Effect::ResizeEmbeddedPty { cols, rows }]
}

pub(super) fn terminal_geometry(state: &AppState) -> (u16, u16) {
    let width = state.terminal.width.max(1);
    let height = state.terminal.height.max(4);
    if state
        .ssh_workspace
        .as_ref()
        .is_some_and(|ws| ws.fullscreen_chrome)
    {
        return (width, height);
    }
    let shelf_w = state
        .ssh_workspace
        .as_ref()
        .filter(|ws| ws.shelf_visible && SshWorkspaceState::side_shelf_layout(width))
        .map(|_| SshWorkspaceState::shelf_width(width))
        .unwrap_or(0);
    let cols = width.saturating_sub(shelf_w).saturating_sub(2).max(20);
    let rows = height.saturating_sub(4).max(3);
    (cols, rows)
}

fn uuid_like() -> String {
    format!(
        "ssh-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_nonzero_keeps_failed_phase() {
        let mut state = AppState {
            ssh_workspace: Some(SshWorkspaceState::new(
                "a".into(),
                "h".into(),
                "u".into(),
                22,
                "a".into(),
                "/usr/bin/ssh".into(),
                vec![],
                vec![],
                Some("a".into()),
                80,
                24,
            )),
            ..AppState::default()
        };
        on_pty_exited(&mut state, Some(255));
        let ws = state.ssh_workspace.as_ref().unwrap();
        assert_eq!(ws.phase, SshConnectionPhase::Failed);
        assert!(ws.status_detail.contains("255"));
    }

    #[test]
    fn exit_zero_records_session_metadata() {
        let mut state = AppState {
            ssh_workspace: Some(SshWorkspaceState::new(
                "a".into(),
                "h".into(),
                "u".into(),
                22,
                "a".into(),
                "/usr/bin/ssh".into(),
                vec![],
                vec![],
                Some("a".into()),
                80,
                24,
            )),
            ..AppState::default()
        };
        let effects = on_pty_exited(&mut state, Some(0));
        assert!(matches!(
            effects.as_slice(),
            [Effect::RecordSshSessionEnded {
                alias,
                exit_code: 0
            }] if alias == "a"
        ));
    }

    #[test]
    fn open_workspace_uses_full_width_while_shelf_hidden() {
        let mut state = AppState {
            terminal: crate::view_model::TerminalState {
                width: 160,
                height: 40,
            },
            ..AppState::default()
        };
        let _ = open_embedded_workspace(
            &mut state,
            "/usr/bin/ssh".into(),
            vec![],
            vec![],
            Some("prod".into()),
            "prod".into(),
            "prod".into(),
            "1.2.3.4".into(),
            "root".into(),
            22,
        );
        let ws = state.ssh_workspace.as_ref().unwrap();
        assert!(!ws.shelf_visible);
        assert_eq!(ws.term_cols, 158);
        let (cols, _) = terminal_geometry(&state);
        assert_eq!(cols, 158);
    }

    #[test]
    fn resize_storm_updates_geometry() {
        let mut state = AppState {
            ssh_workspace: Some(SshWorkspaceState::new(
                "a".into(),
                "h".into(),
                "u".into(),
                22,
                "a".into(),
                "/usr/bin/ssh".into(),
                vec![],
                vec![],
                Some("a".into()),
                80,
                24,
            )),
            route: Route::SshWorkspace,
            ..AppState::default()
        };
        if let Some(ws) = state.ssh_workspace.as_mut() {
            ws.shelf_visible = true;
            ws.phase = SshConnectionPhase::Connected;
        }
        for width in [160u16, 100, 70, 200, 118, 80] {
            state.terminal.width = width;
            state.terminal.height = 40;
            let (cols, rows) = terminal_geometry(&state);
            if let Some(ws) = state.ssh_workspace.as_mut() {
                ws.term_cols = cols;
                ws.term_rows = rows;
            }
            assert!(cols >= 20);
            assert!(rows >= 3);
        }
    }

    #[test]
    fn shelf_toggle_and_leader_have_separate_focus_states() {
        let mut state = AppState {
            terminal: crate::view_model::TerminalState {
                width: 120,
                height: 40,
            },
            ssh_workspace: Some(SshWorkspaceState::new(
                "a".into(),
                "h".into(),
                "u".into(),
                22,
                "a".into(),
                "/usr/bin/ssh".into(),
                vec![],
                vec![],
                Some("a".into()),
                118,
                36,
            )),
            route: Route::SshWorkspace,
            ..AppState::default()
        };

        let _ = toggle_shelf(&mut state);
        let ws = state.ssh_workspace.as_ref().unwrap();
        assert!(ws.shelf_visible);
        assert_eq!(ws.focus, SshWorkspaceFocus::Shelf);
        assert!(!ws.leader_armed);

        let _ = arm_leader(&mut state);
        let ws = state.ssh_workspace.as_ref().unwrap();
        assert!(!ws.shelf_visible);
        assert_eq!(ws.focus, SshWorkspaceFocus::Leader);
        assert!(ws.leader_armed);
    }

    #[test]
    fn fullscreen_geometry_matches_the_borderless_render_area() {
        let mut state = AppState {
            terminal: crate::view_model::TerminalState {
                width: 120,
                height: 40,
            },
            ssh_workspace: Some(SshWorkspaceState::new(
                "a".into(),
                "h".into(),
                "u".into(),
                22,
                "a".into(),
                "/usr/bin/ssh".into(),
                vec![],
                vec![],
                Some("a".into()),
                118,
                36,
            )),
            route: Route::SshWorkspace,
            ..AppState::default()
        };
        state.ssh_workspace.as_mut().unwrap().fullscreen_chrome = true;

        assert_eq!(terminal_geometry(&state), (120, 40));
    }
}
