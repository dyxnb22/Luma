use super::util::{display_width, pad_line_to_width, truncate};
use crate::theme::{Symbols, Theme};
use crate::view_model::AppState;
use luma_domain::ActionRisk;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

pub(super) fn overlay_area(frame_area: Rect, prefer_height: u16) -> Rect {
    let horizontal_gutter = if frame_area.width >= 40 { 4 } else { 0 };
    let max_width = frame_area.width.saturating_sub(horizontal_gutter);
    let width = (frame_area.width.saturating_mul(3) / 4)
        .clamp(36, 104)
        .min(max_width);
    let vertical_gutter = if frame_area.height >= 9 { 4 } else { 0 };
    let max_height = frame_area.height.saturating_sub(vertical_gutter);
    let height = prefer_height.max(5).min(max_height);
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

fn overlay_block(theme: &Theme, symbols: &Symbols) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(symbols.border_type())
        .border_style(with_panel_bg(theme.border(true), theme))
        .style(panel_style(theme))
}

fn overlay_hint(text: String, theme: &Theme) -> Line<'static> {
    Line::from(Span::styled(text, with_panel_bg(theme.keycap(), theme))).right_aligned()
}
pub(super) fn render_overlay_confirm(
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

    let pending = state.actions.pending_action.as_ref();
    let action = pending.map(|p| p.action.label.as_str()).unwrap_or("action");
    let risk = pending.map(|p| &p.action.risk);
    let consequence = match risk {
        Some(ActionRisk::Destructive) => {
            "This cannot be undone from here. Enter runs it; Esc cancels."
        }
        Some(ActionRisk::Confirm) => "Enter runs the action; Esc cancels without changes.",
        _ => "Enter confirms; Esc cancels.",
    };
    let target = pending
        .and_then(|p| {
            state
                .search
                .results
                .items
                .iter()
                .find(|i| i.id.as_str() == p.result_id)
        })
        .map(|i| i.title.as_str())
        .or_else(|| {
            state
                .wordbook
                .review
                .as_ref()
                .filter(|_| pending.is_some())
                .and_then(|review| review.words.get(review.index))
                .map(|word| word.term.as_str())
        })
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
        Line::from(Span::styled(
            format!("  Action: {action}"),
            with_panel_bg(theme.text().add_modifier(Modifier::BOLD), theme),
        )),
        Line::from(Span::styled(
            format!("  Target: {target}"),
            with_panel_bg(theme.accent(), theme),
        )),
        Line::from(Span::styled(
            format!("  {consequence}"),
            with_panel_bg(theme.muted(), theme),
        )),
    ];

    let widget = Paragraph::new(lines)
        .style(panel)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false })
        .block(
            overlay_block(theme, symbols)
                .border_style(title_style)
                .title(Span::styled(" CONFIRM ", title_style))
                .title_bottom(overlay_hint(
                    format!(" Enter confirm {} Esc cancel ", symbols.sep),
                    theme,
                )),
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
    let rows = (state.actions.action_choices.len() as u16)
        .saturating_add(2)
        .max(6);
    let overlay = overlay_area(area, rows.min(16));
    fill_overlay_panel(frame, overlay, theme);
    let panel = panel_style(theme);

    let target = state
        .actions
        .action_result_id
        .as_ref()
        .and_then(|id| {
            state
                .search
                .results
                .items
                .iter()
                .find(|i| i.id.as_str() == id.as_str())
        })
        .map(|i| i.title.clone())
        .unwrap_or_else(|| "item".into());

    let visible_rows = overlay.height.saturating_sub(2) as usize;
    let content_width = overlay.width.saturating_sub(2) as usize;
    let selected_index = state
        .actions
        .action_selected
        .min(state.actions.action_choices.len().saturating_sub(1));
    let start = selected_index
        .saturating_add(1)
        .saturating_sub(visible_rows.max(1));
    let items: Vec<ListItem> = state
        .actions
        .action_choices
        .iter()
        .enumerate()
        .skip(start)
        .take(visible_rows.max(1))
        .map(|(idx, action)| {
            let selected = idx == state.actions.action_selected;
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
            let risk_text = format!("{risk}{confirm}");
            let label = truncate(
                &action.label,
                content_width
                    .saturating_sub(display_width(&risk_text) + 5)
                    .max(6),
                symbols,
            );
            let mut spans = vec![
                Span::styled(" ", style),
                Span::styled(
                    prefix.to_string(),
                    if selected {
                        theme.selected_marker()
                    } else {
                        style
                    },
                ),
                Span::styled(format!(" {label}"), style),
            ];
            let gap = content_width
                .saturating_sub(
                    spans
                        .iter()
                        .map(|span| display_width(span.content.as_ref()))
                        .sum::<usize>()
                        + display_width(&risk_text),
                )
                .max(1);
            spans.push(Span::styled(" ".repeat(gap), row_bg));
            spans.push(Span::styled(risk_text, risk_style));
            pad_line_to_width(&mut spans, content_width, row_bg);
            ListItem::new(Line::from(spans))
        })
        .collect();

    let scroll_hint = if state.actions.action_choices.len() > visible_rows {
        format!(
            " {}{}",
            if start > 0 { symbols.up } else { "" },
            if start + visible_rows < state.actions.action_choices.len() {
                symbols.down
            } else {
                ""
            }
        )
    } else {
        String::new()
    };
    let list = List::new(items).style(panel).block(
        overlay_block(theme, symbols)
            .title(Span::styled(
                format!(" ACTIONS {} {target}{scroll_hint} ", symbols.sep),
                with_panel_bg(theme.title(), theme),
            ))
            .title_bottom(overlay_hint(
                format!(
                    " {}{} move {} Enter run {} Esc back ",
                    symbols.up, symbols.down, symbols.sep, symbols.sep
                ),
                theme,
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
    let lines = state.help_lines();

    let overlay = overlay_area(area, (area.height.saturating_sub(2)).clamp(12, 22));
    fill_overlay_panel(frame, overlay, theme);
    let panel = panel_style(theme);
    let inner_h = overlay.height.saturating_sub(2) as usize;
    let inner_w = overlay.width.saturating_sub(2) as usize;
    let max_scroll = state.help_scroll_max();
    let scroll = state.overlay.help_scroll.min(max_scroll);
    let visible = lines
        .iter()
        .skip(scroll)
        .take(inner_h.max(1))
        .map(|line| styled_help_line(line, inner_w, theme, symbols))
        .collect::<Vec<_>>();
    let position = if max_scroll > 0 {
        format!(
            " {}/{} {}{} ",
            scroll + 1,
            lines.len().max(1),
            if scroll > 0 { symbols.up } else { "" },
            if scroll < max_scroll {
                symbols.down
            } else {
                ""
            }
        )
    } else {
        String::new()
    };
    let widget = Paragraph::new(visible)
        .style(panel)
        .wrap(Wrap { trim: false })
        .block(
            overlay_block(theme, symbols)
                .title(Span::styled(" HELP ", with_panel_bg(theme.title(), theme)))
                .title(
                    Line::from(Span::styled(position, with_panel_bg(theme.muted(), theme)))
                        .right_aligned(),
                )
                .title_bottom(overlay_hint(
                    format!(" PgUp/PgDn page {} Esc back ", symbols.sep),
                    theme,
                )),
        );
    frame.render_widget(widget, overlay);
}

fn styled_help_line(text: &str, width: usize, theme: &Theme, symbols: &Symbols) -> Line<'static> {
    if text.is_empty() {
        return Line::from(Span::styled(String::new(), panel_style(theme)));
    }
    let trimmed = text.trim();
    if trimmed.ends_with(':') {
        return Line::from(Span::styled(
            truncate(text, width, symbols),
            with_panel_bg(theme.section(), theme),
        ));
    }
    if let Some((command, description)) = text.split_once(" — ") {
        let command_budget = (width.saturating_mul(45) / 100).max(12);
        let command = truncate(command, command_budget, symbols);
        let description = truncate(
            description,
            width.saturating_sub(display_width(&command) + 5).max(8),
            symbols,
        );
        return Line::from(vec![
            Span::styled(command, with_panel_bg(theme.keycap(), theme)),
            Span::styled("  —  ", with_panel_bg(theme.muted(), theme)),
            Span::styled(description, with_panel_bg(theme.text(), theme)),
        ]);
    }
    let style = if trimmed.starts_with("Confirm / Destructive") {
        theme.warning()
    } else if text.starts_with("  ") || trimmed.starts_with('(') {
        theme.muted()
    } else {
        theme.text()
    };
    Line::from(Span::styled(
        truncate(text, width, symbols),
        with_panel_bg(style, theme),
    ))
}

pub(super) fn render_overlay_settings(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    symbols: &Symbols,
) {
    dim_backdrop(frame, area, theme);
    let overlay = overlay_area(area, 18);
    fill_overlay_panel(frame, overlay, theme);
    let panel = panel_style(theme);
    let content_width = overlay.width.saturating_sub(2) as usize;
    let fit = |text: String| truncate(&text, content_width, symbols);
    let mut items = Vec::new();
    let projects_line = if state.settings.roots.projects_roots.is_empty() {
        fit(" Projects: (none) · /settings projects-root PATH".into())
    } else {
        fit(format!(
            " Projects: {}",
            state.settings.roots.projects_roots.join(", ")
        ))
    };
    items.push(styled_settings_summary(projects_line, theme));
    items.push(styled_settings_summary(
        fit(format!(
            " Records: {} · /settings records-root PATH",
            state
                .settings
                .values
                .records_root
                .as_deref()
                .unwrap_or("(none)")
        )),
        theme,
    ));
    items.push(styled_settings_summary(
        fit(format!(
            " Clipboard: {}d · /settings clipboard-retention-days N",
            state.settings.values.clipboard_retention_days
        )),
        theme,
    ));
    items.push(styled_settings_summary(
        fit(format!(
            " Secrets lock: {}s · /settings secrets-idle-lock-secs N",
            state.settings.values.secrets_idle_lock_secs
        )),
        theme,
    ));
    items.push(styled_settings_summary(
        fit(format!(
            " Hub windows: {} · /settings hub-windows-max N",
            state.settings.values.hub_windows_max
        )),
        theme,
    ));
    let imported_line = if state.settings.roots.imported_projects.is_empty() {
        fit(" Imported: (none) · /proj add PATH or /proj browse".into())
    } else {
        fit(format!(
            " Imported: {} project(s)",
            state.settings.roots.imported_projects.len()
        ))
    };
    items.push(styled_settings_summary(imported_line, theme));
    if !state.settings.roots.imported_projects.is_empty() {
        for path in state.settings.roots.imported_projects.iter().take(8) {
            items.push(ListItem::new(Span::styled(
                fit(format!("   · {path}")),
                with_panel_bg(theme.muted(), theme),
            )));
        }
        if state.settings.roots.imported_projects.len() > 8 {
            items.push(ListItem::new(Span::styled(
                fit(format!(
                    "   · … {} more",
                    state.settings.roots.imported_projects.len() - 8
                )),
                with_panel_bg(theme.muted(), theme),
            )));
        }
    }
    items.push(ListItem::new(Span::styled(
        fit(" MODULES  ·  Space toggles".into()),
        with_panel_bg(theme.section(), theme),
    )));
    let module_start = items.len();
    if state.settings.modules.is_empty() {
        items.push(ListItem::new(Span::styled(
            fit("  Loading modules…".into()),
            with_panel_bg(theme.muted(), theme),
        )));
    } else {
        for (idx, row) in state.settings.modules.iter().enumerate() {
            let selected = idx == state.settings.selected;
            let prefix = if selected { symbols.selected } else { " " };
            let mark = if row.enabled { "on " } else { "off" };
            let row_bg = if selected {
                Style::default().bg(theme.selected_bg)
            } else {
                Style::default().bg(theme.panel_bg)
            };
            let style = if selected {
                theme.selected_row()
            } else {
                with_panel_bg(theme.text(), theme)
            };
            let state_style = if row.enabled {
                theme.success()
            } else {
                theme.muted()
            }
            .patch(row_bg);
            let detail = truncate(
                &format!("{}  {}", row.name, row.id),
                content_width.saturating_sub(9).max(6),
                symbols,
            );
            let mut spans = vec![
                Span::styled(" ", style),
                Span::styled(
                    prefix.to_string(),
                    if selected {
                        theme.selected_marker()
                    } else {
                        style
                    },
                ),
                Span::styled(format!(" {mark:>3}  "), state_style),
                Span::styled(detail, style),
            ];
            pad_line_to_width(&mut spans, content_width, row_bg);
            items.push(ListItem::new(Line::from(spans)));
        }
    }
    let visible_height = overlay.height.saturating_sub(2) as usize;
    let selected_row = if state.settings.modules.is_empty() {
        0
    } else {
        module_start
            + state
                .settings
                .selected
                .min(state.settings.modules.len() - 1)
    };
    let max_scroll = items.len().saturating_sub(visible_height.max(1));
    let scroll = selected_row
        .saturating_sub(visible_height.saturating_sub(1))
        .min(max_scroll);
    let scroll_hint = if max_scroll == 0 {
        String::new()
    } else {
        format!(
            " {}{}",
            if scroll > 0 { symbols.up } else { "" },
            if scroll < max_scroll {
                symbols.down
            } else {
                ""
            }
        )
    };
    let visible_items = items
        .into_iter()
        .skip(scroll)
        .take(visible_height.max(1))
        .collect::<Vec<_>>();
    let list = List::new(visible_items).style(panel).block(
        overlay_block(theme, symbols)
            .title(Span::styled(
                format!(" SETTINGS  v{}{} ", state.settings.version, scroll_hint),
                with_panel_bg(theme.title(), theme),
            ))
            .title_bottom(overlay_hint(
                format!(" Space toggle {} Esc back ", symbols.sep),
                theme,
            )),
    );
    frame.render_widget(list, overlay);
}

fn styled_settings_summary(text: String, theme: &Theme) -> ListItem<'static> {
    let Some((label, rest)) = text.split_once(':') else {
        return ListItem::new(Span::styled(text, with_panel_bg(theme.muted(), theme)));
    };
    if let Some((value, command)) = rest.split_once(" · ") {
        ListItem::new(Line::from(vec![
            Span::styled(format!("{label}:"), with_panel_bg(theme.section(), theme)),
            Span::styled(value.to_string(), with_panel_bg(theme.text(), theme)),
            Span::styled(
                format!("  ·  {command}"),
                with_panel_bg(theme.muted(), theme),
            ),
        ]))
    } else {
        ListItem::new(Line::from(vec![
            Span::styled(format!("{label}:"), with_panel_bg(theme.section(), theme)),
            Span::styled(rest.to_string(), with_panel_bg(theme.text(), theme)),
        ]))
    }
}

pub(super) fn render_overlay_commands(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    symbols: &Symbols,
) {
    dim_backdrop(frame, area, theme);
    let commands = state.command_palette_rows();
    let overlay = overlay_area(area, (commands.len() as u16).saturating_add(2).clamp(8, 18));
    fill_overlay_panel(frame, overlay, theme);
    let panel = panel_style(theme);
    let visible_rows = overlay.height.saturating_sub(2) as usize;
    let content_width = overlay.width.saturating_sub(2) as usize;
    let selected = state
        .overlay
        .commands_selected
        .min(commands.len().saturating_sub(1));
    let start = selected
        .saturating_add(1)
        .saturating_sub(visible_rows.max(1));
    let command_column = commands
        .iter()
        .map(|entry| display_width(&entry.label))
        .max()
        .unwrap_or(12)
        .min(content_width.saturating_mul(45) / 100)
        .max(12);
    let items: Vec<ListItem> = if commands.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            format!("  No commands match “{}”", state.overlay.commands_filter),
            with_panel_bg(theme.muted(), theme),
        )))]
    } else {
        commands
            .iter()
            .enumerate()
            .skip(start)
            .take(visible_rows.max(1))
            .map(|(idx, entry)| {
                let selected = idx == state.overlay.commands_selected;
                let prefix = if selected { symbols.selected } else { " " };
                let row_bg = if selected {
                    Style::default().bg(theme.selected_bg)
                } else {
                    Style::default().bg(theme.panel_bg)
                };
                let style = if selected {
                    theme.selected_row()
                } else {
                    with_panel_bg(theme.text(), theme)
                };
                let description_style = if selected {
                    Style::default().fg(theme.selected_fg).bg(theme.selected_bg)
                } else {
                    with_panel_bg(theme.muted(), theme)
                };
                let command = truncate(&entry.label, command_column, symbols);
                let description = truncate(
                    &entry.description,
                    content_width.saturating_sub(command_column + 5).max(8),
                    symbols,
                );
                let command_gap = command_column.saturating_sub(display_width(&command)) + 2;
                let mut spans = vec![
                    Span::styled(" ", style),
                    Span::styled(
                        prefix.to_string(),
                        if selected {
                            theme.selected_marker()
                        } else {
                            style
                        },
                    ),
                    Span::styled(
                        format!(" {command}"),
                        if selected {
                            style
                        } else {
                            theme.keycap().bg(theme.panel_bg)
                        },
                    ),
                    Span::styled(" ".repeat(command_gap), row_bg),
                    Span::styled(description, description_style),
                ];
                pad_line_to_width(&mut spans, content_width, row_bg);
                ListItem::new(Line::from(spans))
            })
            .collect()
    };
    let title = if state.overlay.commands_filter.is_empty() {
        " COMMANDS ".into()
    } else {
        format!(
            " COMMANDS  {}  {} ",
            symbols.sep, state.overlay.commands_filter
        )
    };
    let position = if commands.is_empty() {
        " 0 ".to_string()
    } else {
        let has_above = start > 0;
        let has_below = start + visible_rows < commands.len();
        format!(
            " {}/{} {}{} ",
            selected + 1,
            commands.len(),
            if has_above { symbols.up } else { "" },
            if has_below { symbols.down } else { "" }
        )
    };
    let list = List::new(items).style(panel).block(
        overlay_block(theme, symbols)
            .title(Span::styled(title, with_panel_bg(theme.title(), theme)))
            .title(
                Line::from(Span::styled(position, with_panel_bg(theme.muted(), theme)))
                    .right_aligned(),
            )
            .title_bottom(overlay_hint(
                format!(
                    " Type filter {} Enter run {} Esc back ",
                    symbols.sep, symbols.sep
                ),
                theme,
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
    let lines = vec![
        Line::from(Span::styled(" Quit Luma? ", warn)),
        Line::from(Span::styled("", panel)),
        Line::from(Span::styled(
            " The current workbench session will stop.",
            with_panel_bg(theme.muted(), theme),
        )),
    ];
    let widget = Paragraph::new(lines).style(panel).block(
        overlay_block(theme, symbols)
            .border_style(warn)
            .title(Span::styled(" QUIT ", warn))
            .title_bottom(overlay_hint(
                format!(" Enter confirm {} Esc stay ", symbols.sep),
                theme,
            )),
    );
    frame.render_widget(widget, overlay);
}
