//! SSH Workspace render: header + embedded terminal (+ optional shelf chrome).

use crate::ssh_workspace::{SshWorkspaceFocus, SshWorkspaceState};
use crate::theme::{Symbols, Theme};
use crate::view_model::AppState;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub fn render_ssh_workspace(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    _symbols: &Symbols,
) {
    let Some(ws) = state.ssh_workspace.as_ref() else {
        return;
    };

    if ws.fullscreen_chrome {
        render_terminal_pane(frame, area, ws, theme, true);
        return;
    }

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(area);

    let header = Paragraph::new(Line::from(vec![
        Span::styled("SSH Workspace  ", theme.accent()),
        Span::styled(ws.header_text(), theme.text()),
    ]));
    frame.render_widget(header, rows[0]);

    let width = state.terminal.width;
    let show_side = ws.shelf_visible && SshWorkspaceState::side_shelf_layout(width);
    let show_fullpage = ws.shelf_visible && SshWorkspaceState::fullpage_shelf_layout(width);
    let show_overlay = ws.shelf_visible && SshWorkspaceState::overlay_shelf_layout(width);

    if show_fullpage {
        render_shelf_pane(frame, rows[1], ws, theme);
    } else if show_side {
        let shelf_w = SshWorkspaceState::shelf_width(width);
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(72), Constraint::Length(shelf_w)])
            .split(rows[1]);
        render_terminal_pane(frame, cols[0], ws, theme, false);
        render_shelf_pane(frame, cols[1], ws, theme);
    } else {
        render_terminal_pane(frame, rows[1], ws, theme, false);
        if show_overlay {
            let overlay = centered_overlay(rows[1], 40);
            render_shelf_pane(frame, overlay, ws, theme);
        }
    }

    let footer = match ws.focus {
        SshWorkspaceFocus::Terminal => {
            if ws.disconnect_confirm {
                "confirm disconnect: Ctrl+Space d · Esc cancel"
            } else if ws.leader_armed {
                "leader: Space=^Space · f fullscreen · d disconnect · r reconnect · Esc"
            } else {
                "Ctrl+Space commands · Esc leave · r reconnect · l compat · c copy error"
            }
        }
        SshWorkspaceFocus::Leader => {
            "leader: Space=^Space · f fullscreen · d disconnect · r reconnect · Esc"
        }
        SshWorkspaceFocus::Shelf if ws.shelf.filling_params => {
            "Tab fields · type value · Enter preview · Esc terminal"
        }
        SshWorkspaceFocus::Shelf => {
            "Esc terminal · ↑/↓ · Enter preview · c copy · i insert · f favorite · / search"
        }
    };
    frame.render_widget(Paragraph::new(Span::styled(footer, theme.muted())), rows[2]);
}

fn render_terminal_pane(
    frame: &mut Frame<'_>,
    area: Rect,
    ws: &SshWorkspaceState,
    theme: &Theme,
    borderless: bool,
) {
    let block = if borderless {
        Block::default()
    } else {
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border(false))
            .title(" terminal ")
    };
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let lines = if ws.lines.is_empty() {
        vec![Line::from(Span::styled("…", theme.muted()))]
    } else {
        ws.lines.clone()
    };
    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_shelf_pane(frame: &mut Frame<'_>, area: Rect, ws: &SshWorkspaceState, theme: &Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border(true))
        .title(" COMMANDS ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let mut lines: Vec<Line> = Vec::new();
    if ws.shelf.filter_editing || !ws.shelf.filter.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("/{}", ws.shelf.filter),
            theme.accent(),
        )));
    }
    if ws.shelf.filling_params {
        lines.push(Line::from(Span::styled("parameters", theme.muted())));
        for line in ws.shelf.param_form_lines() {
            lines.push(Line::from(Span::styled(line, theme.text())));
        }
        if let Some(preview) = &ws.shelf.preview {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(preview.clone(), theme.muted())));
        }
        frame.render_widget(Paragraph::new(lines), inner);
        return;
    }
    let mut last_group = String::new();
    for (vis_idx, item_idx) in ws.shelf.filtered.iter().enumerate() {
        let Some(item) = ws.shelf.items.get(*item_idx) else {
            continue;
        };
        if item.group != last_group {
            lines.push(Line::from(Span::styled(item.group.clone(), theme.accent())));
            last_group = item.group.clone();
        }
        let marker = if vis_idx == ws.shelf.selected {
            "› "
        } else {
            "  "
        };
        let star = if item.favorite { "★ " } else { "" };
        let risk = match &item.kind {
            crate::ssh_workspace::ShelfItemKind::RemoteCommand { risk, .. }
                if risk != "safe" =>
            {
                format!(" [{risk}]")
            }
            _ => String::new(),
        };
        lines.push(Line::from(format!("{marker}{star}{}{risk}", item.title)));
    }
    if let Some(preview) = &ws.shelf.preview {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("preview", theme.muted())));
        lines.push(Line::from(Span::styled(preview.clone(), theme.text())));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

fn centered_overlay(area: Rect, width: u16) -> Rect {
    let w = width.min(area.width.saturating_sub(2)).max(20);
    let x = area.x + area.width.saturating_sub(w) / 2;
    Rect {
        x,
        y: area.y,
        width: w,
        height: area.height,
    }
}
