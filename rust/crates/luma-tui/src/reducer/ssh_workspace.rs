//! SSH Workspace reducer helpers (pure).

use crate::effect::Effect;
use crate::ssh_workspace::{SshConnectionPhase, SshWorkspaceFocus, SshWorkspaceState};
use crate::view_model::{AppState, FocusZone, Route, StatusTone};

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
    // Header(1) + footer(1) + borders ≈ reserve 4 rows; side shelf may shrink cols.
    let shelf = SshWorkspaceState::side_shelf_layout(width);
    let shelf_w = if shelf {
        SshWorkspaceState::shelf_width(width)
    } else {
        0
    };
    let term_cols = width.saturating_sub(shelf_w).saturating_sub(2).max(20);
    let term_rows = height.saturating_sub(4).max(3);
    state.ssh_workspace = Some(SshWorkspaceState::new(
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
    ));
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
    if code == Some(0) {
        ws.phase = SshConnectionPhase::Disconnected;
        ws.status_detail = "Exited".into();
    } else if let Some(code) = code {
        ws.phase = SshConnectionPhase::Failed;
        ws.status_detail = format!("Exited with code {code}");
        ws.error_summary = ws.status_detail.clone();
    } else {
        ws.phase = SshConnectionPhase::Disconnected;
        ws.status_detail = "Connection lost".into();
        ws.error_summary = ws.status_detail.clone();
    }
    state.status.set(ws.status_detail.clone(), StatusTone::Warning);
    vec![Effect::None]
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
    }
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
    let width = state.terminal.width;
    let Some(ws) = state.ssh_workspace.as_mut() else {
        return vec![Effect::None];
    };
    if SshWorkspaceState::side_shelf_layout(width) {
        ws.shelf_visible = !ws.shelf_visible;
        if !ws.shelf_visible {
            ws.focus = SshWorkspaceFocus::Terminal;
            state.focus = FocusZone::Terminal;
        } else {
            ws.focus = SshWorkspaceFocus::Shelf;
            state.focus = FocusZone::CommandShelf;
        }
    } else {
        // Narrow: open/close overlay or full-page shelf.
        ws.shelf_visible = !ws.shelf_visible;
        ws.focus = if ws.shelf_visible {
            SshWorkspaceFocus::Shelf
        } else {
            SshWorkspaceFocus::Terminal
        };
        state.focus = if ws.shelf_visible {
            FocusZone::CommandShelf
        } else {
            FocusZone::Terminal
        };
    }
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
    format!("ssh-{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_nonzero_keeps_failed_phase() {
        let mut state = AppState::default();
        state.ssh_workspace = Some(SshWorkspaceState::new(
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
        ));
        on_pty_exited(&mut state, Some(255));
        let ws = state.ssh_workspace.as_ref().unwrap();
        assert_eq!(ws.phase, SshConnectionPhase::Failed);
        assert!(ws.status_detail.contains("255"));
    }
}
