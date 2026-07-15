use crate::theme::{Symbols, Theme};
use crate::view_model::AppState;
use luma_domain::ActionRisk;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

pub(super) fn overlay_area(frame_area: Rect, prefer_height: u16) -> Rect {
    let width = (frame_area.width.saturating_mul(2) / 3)
        .max(36)
        .min(frame_area.width.saturating_sub(4));
    let height = prefer_height
        .min(frame_area.height.saturating_sub(4))
        .max(5);
    let x = frame_area.x + (frame_area.width.saturating_sub(width)) / 2;
    let y = frame_area.y + (frame_area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

pub(super) fn dim_backdrop(frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
    let dim = Block::default().style(Style::default().bg(theme.overlay_dim));
    frame.render_widget(dim, area);
}

/// Paint overlay panel with theme background (avoid `Clear`, which uses the terminal default).
pub(super) fn fill_overlay_panel(frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
    frame.render_widget(Block::default().style(panel_style(theme)), area);
}

pub(super) fn panel_style(theme: &Theme) -> Style {
    Style::default().bg(theme.panel_bg).fg(theme.text)
}

pub(super) fn with_panel_bg(style: Style, theme: &Theme) -> Style {
    style.bg(theme.panel_bg)
}
pub(super) fn render_overlay_confirm(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    symbols: &Symbols,
) {
    dim_backdrop(frame, area, theme);
    let overlay = overlay_area(area, 9);
    fill_overlay_panel(frame, overlay, theme);
    let panel = panel_style(theme);

    let pending = state.pending_action.as_ref();
    let action = pending.map(|p| p.action.label.as_str()).unwrap_or("action");
    let risk = pending.map(|p| &p.action.risk);
    let target = pending
        .and_then(|p| {
            state
                .results
                .items
                .iter()
                .find(|i| i.id.as_str() == p.result_id)
        })
        .map(|i| i.title.as_str())
        .unwrap_or("selected item");

    let (title_style, risk_label) = match risk {
        Some(ActionRisk::Destructive) => (theme.destructive(), "DESTRUCTIVE"),
        Some(ActionRisk::Confirm) => (theme.warning(), "CONFIRM"),
        _ => (theme.accent(), "CONFIRM"),
    };
    let title_style = with_panel_bg(title_style, theme);

    let lines = vec![
        Line::from(Span::styled(format!(" {risk_label} "), title_style)),
        Line::from(Span::styled("", panel)),
        Line::from(vec![
            Span::styled("  ", with_panel_bg(theme.muted(), theme)),
            Span::styled(
                action,
                with_panel_bg(theme.text().add_modifier(Modifier::BOLD), theme),
            ),
            Span::styled(" -> ", with_panel_bg(theme.muted(), theme)),
            Span::styled(target, with_panel_bg(theme.accent(), theme)),
        ]),
        Line::from(Span::styled("", panel)),
        Line::from(Span::styled(
            format!("  Enter confirm {} Esc cancel", symbols.sep),
            with_panel_bg(theme.key_hint(), theme),
        )),
    ];

    let widget = Paragraph::new(lines)
        .style(panel)
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(title_style)
                .style(panel)
                .title(Span::styled(" confirm ", title_style)),
        );
    frame.render_widget(widget, overlay);
}

pub(super) fn render_overlay_action_picker(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    symbols: &Symbols,
) {
    dim_backdrop(frame, area, theme);
    let rows = (state.action_choices.len() as u16).saturating_add(2).max(6);
    let overlay = overlay_area(area, rows.min(16));
    fill_overlay_panel(frame, overlay, theme);
    let panel = panel_style(theme);

    let target = state
        .action_result_id
        .as_ref()
        .and_then(|id| {
            state
                .results
                .items
                .iter()
                .find(|i| i.id.as_str() == id.as_str())
        })
        .map(|i| i.title.clone())
        .unwrap_or_else(|| "item".into());

    let items: Vec<ListItem> = state
        .action_choices
        .iter()
        .enumerate()
        .map(|(idx, action)| {
            let selected = idx == state.action_selected;
            let prefix = if selected { symbols.selected } else { " " };
            let row_bg = if selected {
                Style::default().bg(theme.selected_bg)
            } else {
                Style::default().bg(theme.panel_bg)
            };
            let style = if selected {
                theme.selected_row()
            } else {
                theme.row()
            }
            .patch(row_bg);
            let risk_style = match action.risk {
                ActionRisk::Destructive => theme.destructive().patch(row_bg),
                ActionRisk::Confirm => theme.warning().patch(row_bg),
                ActionRisk::Safe => theme.muted().patch(row_bg),
            };
            let risk = match action.risk {
                ActionRisk::Destructive => "destructive",
                ActionRisk::Confirm => "confirm",
                ActionRisk::Safe => "safe",
            };
            let confirm = if action.confirmation {
                format!(" {} asks", symbols.sep)
            } else {
                String::new()
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{prefix} {} ", action.label), style),
                Span::styled(format!("({risk}{confirm})"), risk_style),
            ]))
        })
        .collect();

    let list = List::new(items).style(panel).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(with_panel_bg(theme.border(true), theme))
            .style(panel)
            .title(Span::styled(
                format!(" actions {} {target} ", symbols.sep),
                with_panel_bg(theme.title(), theme),
            )),
    );
    frame.render_widget(list, overlay);
}

pub(super) fn render_overlay_help(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    symbols: &Symbols,
) {
    dim_backdrop(frame, area, theme);
    // Shortcuts first (compact), then enabled modules — config tips last so narrow
    // terminals still see modules after a short scroll.
    let mut lines: Vec<String> = vec![
        "Triggers need a trailing space (`n docker`) · Esc clears · empty Esc quits".to_string(),
        "Enter action · Ctrl-k actions · Ctrl-/ commands · Tab focus · ? help".to_string(),
        format!(
            "{}{} / PgUp PgDn move · Ctrl-u home · Ctrl-w word",
            symbols.up, symbols.down
        ),
        String::new(),
        "Enabled modules:".to_string(),
    ];
    let mut modules: Vec<_> = state.module_catalog.iter().filter(|m| m.enabled).collect();
    modules.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
    });
    if modules.is_empty() {
        lines.push("  (waiting for session catalog)".to_string());
    } else {
        for m in modules {
            let triggers = if m.triggers.is_empty() {
                "—".to_string()
            } else {
                m.triggers.join("/")
            };
            lines.push(format!("  {} · {}", m.display_name, triggers));
        }
    }
    lines.push(String::new());
    lines.push("Config: luma config set --notes-root ~/Notes".to_string());
    lines.push("        luma config set --projects-root ~/dev".to_string());
    lines.push("Confirm / Destructive actions always ask first.".to_string());

    let overlay = overlay_area(area, (area.height.saturating_sub(2)).clamp(12, 22));
    fill_overlay_panel(frame, overlay, theme);
    let panel = panel_style(theme);
    let inner_h = overlay.height.saturating_sub(2) as usize;
    let max_scroll = lines.len().saturating_sub(inner_h.max(1));
    let scroll = state.help_scroll.min(max_scroll);
    let visible = lines
        .iter()
        .skip(scroll)
        .take(inner_h.max(1))
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    let title = if max_scroll > 0 {
        format!(
            " help {} {}/{} ",
            symbols.sep,
            scroll + 1,
            lines.len().max(1)
        )
    } else {
        " help ".to_string()
    };
    let widget = Paragraph::new(visible)
        .style(panel)
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(with_panel_bg(theme.border(true), theme))
                .style(panel)
                .title(Span::styled(title, with_panel_bg(theme.title(), theme))),
        );
    frame.render_widget(widget, overlay);
}

pub(super) fn render_overlay_settings(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    symbols: &Symbols,
) {
    dim_backdrop(frame, area, theme);
    let overlay = overlay_area(area, 16);
    fill_overlay_panel(frame, overlay, theme);
    let panel = panel_style(theme);
    let mut items = Vec::new();
    let notes_line = match &state.settings_roots.notes_root {
        Some(root) if !root.is_empty() => format!(" Notes root: {root}"),
        _ => " Notes root: (not set) · luma config set --notes-root ~/Notes".into(),
    };
    let projects_line = if state.settings_roots.projects_roots.is_empty() {
        " Projects: (none) · luma config set --projects-root ~/dev".into()
    } else {
        format!(
            " Projects: {}",
            state.settings_roots.projects_roots.join(", ")
        )
    };
    items.push(ListItem::new(Span::styled(
        notes_line,
        with_panel_bg(theme.muted(), theme),
    )));
    items.push(ListItem::new(Span::styled(
        projects_line,
        with_panel_bg(theme.muted(), theme),
    )));
    items.push(ListItem::new(Span::styled(
        " — modules (Space toggles) —",
        with_panel_bg(theme.muted(), theme),
    )));
    if state.settings_modules.is_empty() {
        items.push(ListItem::new(Span::styled(
            "  Loading modules…",
            with_panel_bg(theme.muted(), theme),
        )));
    } else {
        for (idx, row) in state.settings_modules.iter().enumerate() {
            let selected = idx == state.settings_selected;
            let prefix = if selected { symbols.selected } else { " " };
            let mark = if row.enabled { "[on] " } else { "[off]" };
            let style = if selected {
                theme.selected_row()
            } else {
                with_panel_bg(theme.text(), theme)
            };
            items.push(ListItem::new(Span::styled(
                format!(" {prefix} {mark} {}  ({})", row.name, row.id),
                style,
            )));
        }
    }
    let list = List::new(items).style(panel).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(with_panel_bg(theme.border(true), theme))
            .style(panel)
            .title(Span::styled(
                format!(" settings v{} ", state.settings_version),
                with_panel_bg(theme.title(), theme),
            )),
    );
    frame.render_widget(list, overlay);
}

pub(super) fn render_overlay_commands(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    symbols: &Symbols,
) {
    dim_backdrop(frame, area, theme);
    let overlay = overlay_area(area, 10);
    fill_overlay_panel(frame, overlay, theme);
    let panel = panel_style(theme);
    let commands = [
        ("settings", "Open module settings"),
        ("help", "Keyboard help"),
        ("quit", "Quit Luma"),
    ];
    let items: Vec<ListItem> = commands
        .iter()
        .enumerate()
        .map(|(idx, (name, desc))| {
            let selected = idx == state.commands_selected;
            let prefix = if selected { symbols.selected } else { " " };
            let style = if selected {
                theme.selected_row()
            } else {
                with_panel_bg(theme.text(), theme)
            };
            let muted = if selected {
                theme.selected_row()
            } else {
                with_panel_bg(theme.muted(), theme)
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {prefix} :{name}  "), style),
                Span::styled((*desc).to_string(), muted),
            ]))
        })
        .collect();
    let list = List::new(items).style(panel).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(with_panel_bg(theme.border(true), theme))
            .style(panel)
            .title(Span::styled(
                " commands ",
                with_panel_bg(theme.title(), theme),
            )),
    );
    frame.render_widget(list, overlay);
}

pub(super) fn render_overlay_quit(
    frame: &mut Frame<'_>,
    area: Rect,
    theme: &Theme,
    symbols: &Symbols,
) {
    dim_backdrop(frame, area, theme);
    let overlay = overlay_area(area, 7);
    fill_overlay_panel(frame, overlay, theme);
    let panel = panel_style(theme);
    let warn = with_panel_bg(theme.warning().add_modifier(Modifier::BOLD), theme);
    let hint = with_panel_bg(theme.key_hint(), theme);
    let lines = vec![
        Line::from(Span::styled(" Quit Luma? ", warn)),
        Line::from(Span::styled("", panel)),
        Line::from(Span::styled(
            format!("  Enter confirm {} Esc stay", symbols.sep),
            hint,
        )),
    ];
    let widget = Paragraph::new(lines).style(panel).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(with_panel_bg(theme.border(true), theme))
            .style(panel)
            .title(Span::styled(" quit ", warn)),
    );
    frame.render_widget(widget, overlay);
}
