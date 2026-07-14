use crate::theme::{module_glyph, module_label, ResultKindVisual, Symbols, Theme};
use crate::view_model::{AppState, Route, StatusTone};
use luma_domain::{ActionRisk, SearchItem};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Pure projection. Must not mutate state, start tasks, or read the environment.
pub fn render(frame: &mut Frame<'_>, state: &AppState) {
    render_with(frame, state, &state.theme, &state.symbols);
}

fn render_with(frame: &mut Frame<'_>, state: &AppState, theme: &Theme, symbols: &Symbols) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(3),
        ])
        .split(area);

    let prompt_focused = matches!(state.focus, crate::view_model::FocusZone::Prompt)
        && matches!(state.route, Route::Search);
    render_prompt(frame, chunks[0], state, theme, symbols, prompt_focused);

    let body = chunks[1];
    if state.preview_side_by_side() {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
            .split(body);
        render_results(frame, cols[0], state, theme, symbols);
        render_preview(frame, cols[1], state, theme, symbols);
    } else if state.preview_stacked() {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(8)])
            .split(body);
        render_results(frame, rows[0], state, theme, symbols);
        render_preview(frame, rows[1], state, theme, symbols);
    } else {
        render_results(frame, body, state, theme, symbols);
    }
    render_status(frame, chunks[2], state, theme, symbols);

    match state.route {
        Route::Search => {}
        Route::Help => render_overlay_help(frame, area, theme, symbols),
        Route::Doctor => render_overlay_doctor(frame, area, state, theme, symbols),
        Route::Settings => render_overlay_settings(frame, area, state, theme, symbols),
        Route::Commands => render_overlay_commands(frame, area, state, theme, symbols),
        Route::QuitConfirm => render_overlay_quit(frame, area, theme, symbols),
        Route::ConfirmAction => render_overlay_confirm(frame, area, state, theme, symbols),
        Route::ActionPicker => render_overlay_action_picker(frame, area, state, theme, symbols),
    }
}

fn render_prompt(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    symbols: &Symbols,
    focused: bool,
) {
    let cursor = if focused { symbols.cursor } else { " " };
    let mut chars = state.prompt.chars();
    let before: String = chars.by_ref().take(state.prompt_cursor).collect();
    let after: String = chars.collect();
    let line = Line::from(vec![
        Span::styled("  ", theme.muted()),
        Span::styled(before, theme.text()),
        Span::styled(cursor, theme.accent()),
        Span::styled(after, theme.text()),
    ]);
    let widget = Paragraph::new(line).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border(focused))
            .title(Span::styled(" Luma ", theme.title())),
    );
    frame.render_widget(widget, area);
}

fn render_results(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    symbols: &Symbols,
) {
    use crate::view_model::FocusZone;

    let list_focused =
        matches!(state.focus, FocusZone::List) && matches!(state.route, Route::Search);
    let inner_height = area.height.saturating_sub(2) as usize;
    let inner_width = area.width.saturating_sub(2);
    let rows_capacity = (inner_height / 2).max(1);
    let total = state.results.items.len();
    let scroll = if total == 0 {
        0
    } else {
        state
            .results
            .scroll
            .min(total.saturating_sub(rows_capacity.min(total)))
    };
    let selected_idx = state.results.selected_index().unwrap_or(0);
    let visible_count = if total == 0 {
        0
    } else {
        rows_capacity.min(total.saturating_sub(scroll))
    };
    let has_above = scroll > 0;
    let has_below = scroll + visible_count < total;

    let mut items: Vec<ListItem> = Vec::new();
    if state.results.items.is_empty() {
        if state.prompt.trim().is_empty() {
            items.extend(hub_list_items(state, theme, symbols));
        } else {
            items.push(empty_state_item(state, theme, symbols));
        }
    } else {
        let query = highlight_query(&state.prompt);
        for item in state.results.items.iter().skip(scroll).take(visible_count) {
            let selected = state
                .results
                .selected_id
                .as_deref()
                .is_some_and(|id| id == item.id.as_str());
            items.push(result_row(
                item,
                selected,
                inner_width,
                &query,
                theme,
                symbols,
                &state.module_labels,
            ));
        }
    }

    let title = if state.results.items.is_empty() {
        if state.prompt.trim().is_empty() {
            " hub ".to_string()
        } else {
            " results ".to_string()
        }
    } else {
        let mut scroll_marks = String::new();
        if has_above {
            scroll_marks.push_str(symbols.up);
        }
        if has_below {
            scroll_marks.push_str(symbols.down);
        }
        let scroll_part = if scroll_marks.is_empty() {
            String::new()
        } else {
            format!(" {scroll_marks}")
        };
        format!(
            " results {} {}/{}{scroll_part} ",
            symbols.sep,
            selected_idx + 1,
            total
        )
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border(list_focused))
            .title(Span::styled(title, theme.muted())),
    );
    frame.render_widget(list, area);
}

fn hub_list_items(state: &AppState, theme: &Theme, symbols: &Symbols) -> Vec<ListItem<'static>> {
    let rows = state.hub_rows();
    if rows.is_empty() {
        return vec![ListItem::new(vec![
            Line::from(Span::styled("  Waiting for modules…", theme.muted())),
            Line::from(Span::styled(
                "  Session warms in the background",
                theme.key_hint(),
            )),
        ])];
    }
    let mut out = Vec::new();
    let mut shown_pins = false;
    let mut shown_modules = false;
    let viewport = state.results.viewport_rows.max(1);
    let end = (state.hub_scroll + viewport).min(rows.len());
    let window = &rows[state.hub_scroll..end];
    for (idx, (kind, _id, title, query)) in window
        .iter()
        .enumerate()
        .map(|(i, r)| (state.hub_scroll + i, r))
    {
        if kind == "pin" && !shown_pins && state.hub_scroll == 0 {
            shown_pins = true;
            out.push(ListItem::new(vec![
                Line::from(Span::styled("  Pinned", theme.title())),
                Line::from(Span::styled(
                    "  Enter selects pin in clipboard · ↑↓ move",
                    theme.key_hint(),
                )),
            ]));
        }
        if kind == "module" && !shown_modules {
            shown_modules = true;
            out.push(ListItem::new(vec![
                Line::from(Span::styled("  Modules", theme.title())),
                Line::from(Span::styled(
                    "  Enter opens trigger · ↑↓ move",
                    theme.key_hint(),
                )),
            ]));
        }
        let selected = idx == state.hub_selected;
        let prefix = if selected { symbols.selected } else { " " };
        let style = if selected {
            theme.selected_row()
        } else {
            theme.text()
        };
        let muted = if selected {
            theme.selected_row()
        } else {
            theme.muted()
        };
        let right = if kind == "pin" {
            "pin".to_string()
        } else {
            query.clone()
        };
        out.push(ListItem::new(vec![
            Line::from(vec![
                Span::styled(format!(" {prefix} {title}"), style),
                Span::styled(format!("  {right}"), muted),
            ]),
            Line::from(""),
        ]));
    }
    out
}

fn render_preview(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    symbols: &Symbols,
) {
    use crate::view_model::FocusZone;

    let focused = matches!(state.focus, FocusZone::Preview) && matches!(state.route, Route::Search);
    let Some(item) = state
        .results
        .selected_id
        .as_ref()
        .and_then(|id| state.results.items.iter().find(|i| i.id.as_str() == id))
    else {
        let widget = Paragraph::new(Span::styled("  No selection", theme.muted())).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border(focused))
                .title(Span::styled(" preview ", theme.muted())),
        );
        frame.render_widget(widget, area);
        return;
    };

    let module = module_label(item.module_id.as_str(), &state.module_labels);
    let kind = ResultKindVisual::from_kind(&item.kind);
    let risk = match item.primary_action.risk {
        ActionRisk::Safe => "safe",
        ActionRisk::Confirm => "confirm",
        ActionRisk::Destructive => "destructive",
    };
    let mut lines = vec![
        Line::from(Span::styled(format!("  {}", item.title), theme.title())),
        Line::from(Span::styled(
            format!(
                "  {} {} {}",
                module,
                symbols.sep,
                kind.badge().unwrap_or(item.kind.as_str())
            ),
            theme.muted(),
        )),
    ];
    if let Some(sub) = &item.subtitle {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(format!("  {sub}"), theme.text())));
    }
    lines.push(Line::from(""));
    if state.preview_result_id.as_deref() == Some(item.id.as_str()) {
        if let Some(body) = &state.preview_body {
            let body_lines: Vec<&str> = body.lines().collect();
            let header_lines = lines.len();
            let visible = (area.height as usize)
                .saturating_sub(header_lines + 2)
                .max(1);
            let max_scroll = body_lines.len().saturating_sub(visible);
            let scroll = state.preview_scroll.min(max_scroll);
            for line in body_lines.into_iter().skip(scroll).take(visible) {
                lines.push(Line::from(Span::styled(format!("  {line}"), theme.text())));
            }
            if max_scroll > 0 {
                lines.push(Line::from(Span::styled(
                    format!("  … {}/{}", scroll + 1, max_scroll + 1),
                    theme.muted(),
                )));
            }
        }
    } else {
        lines.push(Line::from(Span::styled(
            format!("  Loading preview{}", symbols.ellipsis),
            theme.muted(),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("  Actions", theme.muted())));
    lines.push(Line::from(Span::styled(
        format!(
            "  {} {} ({})",
            symbols.enter, item.primary_action.label, risk
        ),
        theme.action_hint(),
    )));
    for sec in &item.secondary_actions {
        lines.push(Line::from(Span::styled(
            format!("  · {}", sec.label),
            theme.key_hint(),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Ctrl-k more actions",
        theme.key_hint(),
    )));

    let widget = Paragraph::new(lines).wrap(Wrap { trim: false }).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border(focused))
            .title(Span::styled(" preview ", theme.muted())),
    );
    frame.render_widget(widget, area);
}

fn empty_state_item(state: &AppState, theme: &Theme, symbols: &Symbols) -> ListItem<'static> {
    let (title, detail) =
        if state.active_request.is_some() && state.status.tone == StatusTone::Progress {
            (
                format!("Searching{}", symbols.ellipsis),
                "Results appear as modules respond".to_string(),
            )
        } else if state.prompt.trim().is_empty() {
            (
                "Type to search".to_string(),
                format!(
                    "Try: app safari {} n meeting {} clip",
                    symbols.sep, symbols.sep
                ),
            )
        } else if let Some(hint) = empty_hint_for_prompt(state) {
            ("No results".to_string(), hint)
        } else {
            (
                "No results".to_string(),
                "Adjust the query or try another module trigger".to_string(),
            )
        };
    ListItem::new(vec![
        Line::from(Span::styled(format!("  {title}"), theme.muted())),
        Line::from(Span::styled(format!("  {detail}"), theme.key_hint())),
    ])
}

/// Prefer the targeted module's `empty_hint` when the prompt starts with its trigger.
fn empty_hint_for_prompt(state: &AppState) -> Option<String> {
    let token = state.prompt.split_whitespace().next()?.to_ascii_lowercase();
    state
        .module_catalog
        .iter()
        .find(|m| m.enabled && m.triggers.iter().any(|t| t.eq_ignore_ascii_case(&token)))
        .and_then(|m| m.empty_hint.clone())
}

fn result_row(
    item: &SearchItem,
    selected: bool,
    width: u16,
    query: &str,
    theme: &Theme,
    symbols: &Symbols,
    module_labels: &std::collections::HashMap<String, String>,
) -> ListItem<'static> {
    let kind = ResultKindVisual::from_kind(&item.kind);
    let glyph = module_glyph(item.module_id.as_str());
    let module = module_label(item.module_id.as_str(), module_labels);
    let action = match kind {
        ResultKindVisual::Warming => format!("{} Wait", symbols.ellipsis),
        _ => format!("{} {}", symbols.enter, item.primary_action.label),
    };
    let prefix = if selected { symbols.selected } else { " " };
    let row_bg = if selected {
        Style::default().bg(theme.selected_bg)
    } else {
        Style::default()
    };
    let base = theme.kind_style(kind, selected).patch(row_bg);
    let muted = if selected {
        Style::default().fg(theme.selected_fg).bg(theme.selected_bg)
    } else {
        theme.muted()
    };
    let hint = match kind {
        ResultKindVisual::Permission => {
            theme.permission().patch(row_bg).add_modifier(if selected {
                Modifier::BOLD
            } else {
                Modifier::empty()
            })
        }
        ResultKindVisual::Unavailable | ResultKindVisual::NotConfigured => {
            theme.warning().patch(row_bg).add_modifier(if selected {
                Modifier::BOLD
            } else {
                Modifier::empty()
            })
        }
        ResultKindVisual::Warming => muted,
        ResultKindVisual::Normal => {
            if selected {
                Style::default()
                    .fg(theme.accent)
                    .bg(theme.selected_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                theme.action_hint()
            }
        }
    };
    let badge = if selected {
        Style::default().fg(theme.muted).bg(theme.selected_bg)
    } else {
        theme.module_badge()
    };

    let kind_badge = kind.badge().unwrap_or("");
    let prefix_w = display_width(&format!("{prefix} {glyph} "));
    let right_w = display_width(&format!(
        "{} {module}  {action}",
        if kind_badge.is_empty() {
            String::new()
        } else {
            format!(" {kind_badge}")
        }
    ));
    let title_budget = (width as usize).saturating_sub(prefix_w + right_w).max(8);
    let title = truncate(&item.title, title_budget, symbols);
    let subtitle = item
        .subtitle
        .as_deref()
        .map(|s| truncate(s, width.saturating_sub(4) as usize, symbols))
        .unwrap_or_default();

    let mut title_spans = vec![Span::styled(format!("{prefix} {glyph} "), base)];
    title_spans.extend(highlighted_spans(
        &pad_right(&title, title_budget),
        query,
        base,
        theme.match_highlight(selected).patch(row_bg),
    ));
    if !kind_badge.is_empty() {
        title_spans.push(Span::styled(format!(" {kind_badge}"), hint));
    }
    title_spans.push(Span::styled(format!(" {module}"), badge));
    title_spans.push(Span::styled(format!("  {action}"), hint));
    pad_line_to_width(&mut title_spans, width as usize, row_bg);

    let mut sub_spans = if subtitle.is_empty() {
        vec![Span::styled("    ", muted)]
    } else {
        vec![Span::styled(format!("    {subtitle}"), muted)]
    };
    pad_line_to_width(&mut sub_spans, width as usize, row_bg);

    ListItem::new(vec![Line::from(title_spans), Line::from(sub_spans)])
}

fn pad_line_to_width(spans: &mut Vec<Span<'static>>, width: usize, fill: Style) {
    let used: usize = spans
        .iter()
        .map(|s| display_width(s.content.as_ref()))
        .sum();
    if used < width {
        spans.push(Span::styled(" ".repeat(width - used), fill));
    }
}

fn highlight_query(prompt: &str) -> String {
    const TRIGGERS: &[&str] = &[
        "app",
        "apps",
        "clip",
        "cb",
        "n",
        "note",
        "notes",
        "ql",
        "quicklinks",
        "s",
        "snip",
        "proj",
        "project",
        "kill",
        "quit",
        "k",
        "sec",
        "secret",
        "secrets",
        "fake",
        "echo",
        "p",
    ];
    let tokens: Vec<&str> = prompt.split_whitespace().collect();
    if tokens.is_empty() {
        return String::new();
    }
    let start = if TRIGGERS.iter().any(|t| tokens[0].eq_ignore_ascii_case(t)) {
        1
    } else {
        0
    };
    tokens[start..].join(" ")
}

fn highlighted_spans(
    text: &str,
    query: &str,
    normal: Style,
    highlight: Style,
) -> Vec<Span<'static>> {
    if query.trim().is_empty() || text.is_empty() {
        return vec![Span::styled(text.to_string(), normal)];
    }

    let chars: Vec<char> = text.chars().collect();
    let lower_chars: Vec<char> = chars
        .iter()
        .map(|c| c.to_lowercase().next().unwrap_or(*c))
        .collect();
    let mut marks = vec![false; chars.len()];

    for needle in query.split_whitespace().filter(|n| !n.is_empty()) {
        let needle_chars: Vec<char> = needle.to_lowercase().chars().collect();
        if needle_chars.is_empty() {
            continue;
        }
        let mut i = 0;
        while i + needle_chars.len() <= lower_chars.len() {
            if lower_chars[i..i + needle_chars.len()] == needle_chars[..] {
                for m in &mut marks[i..i + needle_chars.len()] {
                    *m = true;
                }
                i += needle_chars.len();
            } else {
                i += 1;
            }
        }
    }

    let mut spans = Vec::new();
    let mut current = String::new();
    let mut current_hl = marks.first().copied().unwrap_or(false);
    for (ch, &hl) in chars.iter().zip(marks.iter()) {
        if hl != current_hl && !current.is_empty() {
            spans.push(Span::styled(
                std::mem::take(&mut current),
                if current_hl { highlight } else { normal },
            ));
            current_hl = hl;
        }
        current.push(*ch);
    }
    if !current.is_empty() {
        spans.push(Span::styled(
            current,
            if current_hl { highlight } else { normal },
        ));
    }
    if spans.is_empty() {
        spans.push(Span::styled(text.to_string(), normal));
    }
    spans
}

fn render_status(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    symbols: &Symbols,
) {
    let status_style = status_style(state.status.tone, theme);
    let hints = match state.route {
        Route::Search => {
            let nav = if state.focus == crate::view_model::FocusZone::Preview {
                "PgUp/Dn scroll preview"
            } else {
                "PgUp/Dn"
            };
            format!(
                "{}{} {nav} {} Enter {} Ctrl-k Actions {} Tab Focus {} Esc up/clear {} ? Help",
                symbols.up,
                symbols.down,
                symbols.sep,
                symbols.sep,
                symbols.sep,
                symbols.sep,
                symbols.sep
            )
        }
        Route::ActionPicker => format!(
            "{}{} 1-9 {} Enter Run {} Esc Back",
            symbols.up, symbols.down, symbols.sep, symbols.sep
        ),
        Route::Settings => format!(
            "{}{} Space Toggle {} Esc Back",
            symbols.up, symbols.down, symbols.sep
        ),
        Route::Commands => format!("Enter Run {} Esc Back", symbols.sep),
        Route::ConfirmAction | Route::QuitConfirm => {
            format!("Enter Confirm {} Esc Cancel", symbols.sep)
        }
        Route::Help => "Esc Back · type to search".to_string(),
        Route::Doctor => format!(
            "{}{} scroll {} Esc Back",
            symbols.up, symbols.down, symbols.sep
        ),
    };
    let count = if state.results.items.is_empty() {
        String::new()
    } else {
        let mut seen = Vec::new();
        for item in &state.results.items {
            let label = module_label(item.module_id.as_str(), &state.module_labels);
            if !seen.contains(&label) {
                seen.push(label);
            }
        }
        let module_part = if seen.is_empty() {
            String::new()
        } else {
            format!(" {} {}", symbols.sep, seen.join(", "))
        };
        format!("{} results{module_part}   ", state.results.items.len())
    };

    let line = Line::from(vec![
        Span::styled(format!(" {}  ", state.status.text), status_style),
        Span::styled(count, theme.muted()),
        Span::styled(hints, theme.key_hint()),
    ]);
    let widget = Paragraph::new(line).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border(false))
            .title(Span::styled(" status ", theme.muted())),
    );
    frame.render_widget(widget, area);
}

fn status_style(tone: StatusTone, theme: &Theme) -> Style {
    match tone {
        StatusTone::Neutral => theme.text(),
        StatusTone::Success => theme.success(),
        StatusTone::Progress => theme.accent(),
        StatusTone::Warning => theme.warning(),
        StatusTone::Error => theme.error(),
        StatusTone::Permission => theme.permission(),
    }
}

fn overlay_area(frame_area: Rect, prefer_height: u16) -> Rect {
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

fn dim_backdrop(frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
    let dim = Block::default().style(Style::default().bg(theme.overlay_dim));
    frame.render_widget(dim, area);
}

fn render_overlay_confirm(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    symbols: &Symbols,
) {
    dim_backdrop(frame, area, theme);
    let overlay = overlay_area(area, 9);
    frame.render_widget(Clear, overlay);

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

    let lines = vec![
        Line::from(Span::styled(format!(" {risk_label} "), title_style)),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", theme.muted()),
            Span::styled(action, theme.text().add_modifier(Modifier::BOLD)),
            Span::styled(" -> ", theme.muted()),
            Span::styled(target, theme.accent()),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            format!("  Enter confirm {} Esc cancel", symbols.sep),
            theme.key_hint(),
        )),
    ];

    let widget = Paragraph::new(lines).alignment(Alignment::Left).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(title_style)
            .title(Span::styled(" confirm ", title_style)),
    );
    frame.render_widget(widget, overlay);
}

fn render_overlay_action_picker(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    symbols: &Symbols,
) {
    dim_backdrop(frame, area, theme);
    let rows = (state.action_choices.len() as u16).saturating_add(2).max(6);
    let overlay = overlay_area(area, rows.min(16));
    frame.render_widget(Clear, overlay);

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
                Style::default()
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

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border(true))
            .title(Span::styled(
                format!(" actions {} {target} ", symbols.sep),
                theme.title(),
            )),
    );
    frame.render_widget(list, overlay);
}

fn render_overlay_help(frame: &mut Frame<'_>, area: Rect, theme: &Theme, symbols: &Symbols) {
    dim_backdrop(frame, area, theme);
    let overlay = overlay_area(area, 18);
    frame.render_widget(Clear, overlay);
    let text = [
        "Type to search · Left/Right/Home/End move cursor".to_string(),
        "Ctrl-u clear to start / home · Ctrl-w delete word".to_string(),
        "Ctrl-p/n query history (prompt focused)".to_string(),
        format!(
            "{}{} / PgUp PgDn  move selection (scroll preview when focused)",
            symbols.up, symbols.down
        ),
        "Enter  primary action · empty Enter opens Hub trigger".to_string(),
        "Ctrl-k  action list · Ctrl-/ command palette".to_string(),
        "Tab  cycle focus (prompt / list / preview)".to_string(),
        "Esc  browse up / clear · Ctrl-u home · empty Esc quit confirm".to_string(),
        "?  help · :doctor / :settings / :commands".to_string(),
        "Preview: side-by-side when wide (≥100); stacked when tall (≥28 rows)".to_string(),
        "Ctrl-C  quit confirm · Enter exits".to_string(),
        String::new(),
        "Configure Notes: luma config set --notes-root ~/Notes".to_string(),
        "Configure Projects: luma config set --projects-root ~/dev".to_string(),
        "Confirm / Destructive actions always ask first.".to_string(),
    ]
    .join("\n");
    let widget = Paragraph::new(text).wrap(Wrap { trim: false }).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border(true))
            .title(Span::styled(" help ", theme.title())),
    );
    frame.render_widget(widget, overlay);
}

fn render_overlay_doctor(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    symbols: &Symbols,
) {
    dim_backdrop(frame, area, theme);
    let overlay = overlay_area(area, (area.height.saturating_sub(4)).clamp(8, 20));
    frame.render_widget(Clear, overlay);
    let full = state.doctor_diagnostic.as_ref().map_or_else(
        || format!("Waiting for engine diagnostics{}", symbols.ellipsis),
        |diagnostic| {
            serde_json::to_string_pretty(diagnostic).unwrap_or_else(|_| diagnostic.to_string())
        },
    );
    let lines: Vec<&str> = full.lines().collect();
    let inner_h = overlay.height.saturating_sub(2) as usize;
    let max_scroll = lines.len().saturating_sub(inner_h.max(1));
    let scroll = state.doctor_scroll.min(max_scroll);
    let visible: String = lines
        .iter()
        .skip(scroll)
        .take(inner_h.max(1))
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    let title = if max_scroll > 0 {
        format!(
            " doctor {} {}/{} ",
            symbols.sep,
            scroll + 1,
            lines.len().max(1)
        )
    } else {
        " doctor ".to_string()
    };
    let widget = Paragraph::new(visible).wrap(Wrap { trim: false }).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border(true))
            .title(Span::styled(title, theme.title())),
    );
    frame.render_widget(widget, overlay);
}

fn render_overlay_settings(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    symbols: &Symbols,
) {
    dim_backdrop(frame, area, theme);
    let overlay = overlay_area(area, 16);
    frame.render_widget(Clear, overlay);
    let mut items = Vec::new();
    if state.settings_modules.is_empty() {
        items.push(ListItem::new(Span::styled(
            "  Loading modules…",
            theme.muted(),
        )));
    } else {
        for (idx, row) in state.settings_modules.iter().enumerate() {
            let selected = idx == state.settings_selected;
            let prefix = if selected { symbols.selected } else { " " };
            let mark = if row.enabled { "[on] " } else { "[off]" };
            let style = if selected {
                theme.selected_row()
            } else {
                theme.text()
            };
            items.push(ListItem::new(Span::styled(
                format!(" {prefix} {mark} {}  ({})", row.name, row.id),
                style,
            )));
        }
    }
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border(true))
            .title(Span::styled(
                format!(" settings v{} ", state.settings_version),
                theme.title(),
            )),
    );
    frame.render_widget(list, overlay);
}

fn render_overlay_commands(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    symbols: &Symbols,
) {
    dim_backdrop(frame, area, theme);
    let overlay = overlay_area(area, 10);
    frame.render_widget(Clear, overlay);
    let commands = [
        ("settings", "Open module settings"),
        ("doctor", "Run diagnostics"),
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
                theme.text()
            };
            let muted = if selected {
                theme.selected_row()
            } else {
                theme.muted()
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {prefix} :{name}  "), style),
                Span::styled((*desc).to_string(), muted),
            ]))
        })
        .collect();
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border(true))
            .title(Span::styled(" commands ", theme.title())),
    );
    frame.render_widget(list, overlay);
}

fn render_overlay_quit(frame: &mut Frame<'_>, area: Rect, theme: &Theme, symbols: &Symbols) {
    dim_backdrop(frame, area, theme);
    let overlay = overlay_area(area, 7);
    frame.render_widget(Clear, overlay);
    let lines = vec![
        Line::from(Span::styled(" Quit Luma? ", theme.warning())),
        Line::from(""),
        Line::from(Span::styled(
            format!("  Enter confirm {} Esc stay", symbols.sep),
            theme.key_hint(),
        )),
    ];
    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.warning())
            .title(Span::styled(" quit ", theme.warning())),
    );
    frame.render_widget(widget, overlay);
}

fn display_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

fn truncate(s: &str, max: usize, symbols: &Symbols) -> String {
    if max == 0 {
        return String::new();
    }
    if display_width(s) <= max {
        return s.to_string();
    }
    let ell = symbols.ellipsis;
    let ell_w = display_width(ell).max(1);
    if max <= ell_w {
        return ell.chars().take(1).collect();
    }
    let keep = max - ell_w;
    let mut out = String::new();
    let mut w = 0;
    for ch in s.chars() {
        let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
        if w + cw > keep {
            break;
        }
        out.push(ch);
        w += cw;
    }
    out.push_str(ell);
    out
}

fn pad_right(s: &str, width: usize) -> String {
    let w = display_width(s);
    if w >= width {
        s.to_string()
    } else {
        format!("{s}{}", " ".repeat(width - w))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{Symbols, Theme, ThemeMode};
    use crate::view_model::ResultsView;
    use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ModuleId, ResultId, SearchItem};
    use ratatui::backend::TestBackend;
    use ratatui::style::{Color, Modifier};
    use ratatui::Terminal;

    fn sample_item(id: &str, title: &str, module: &str, subtitle: &str) -> SearchItem {
        SearchItem {
            id: ResultId::new(id),
            module_id: ModuleId::new(module),
            title: title.into(),
            subtitle: Some(subtitle.into()),
            kind: "app".into(),
            score: 10.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("launch"),
                label: "Launch".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
        }
    }

    fn sample_kind(
        id: &str,
        title: &str,
        module: &str,
        kind: &str,
        subtitle: &str,
        action: &str,
    ) -> SearchItem {
        SearchItem {
            id: ResultId::new(id),
            module_id: ModuleId::new(module),
            title: title.into(),
            subtitle: Some(subtitle.into()),
            kind: kind.into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("act"),
                label: action.into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
        }
    }

    fn state_with_results() -> AppState {
        AppState {
            theme: Theme::dark(),
            symbols: Symbols::unicode(),
            prompt: "app saf".into(),
            status: crate::view_model::StatusLine {
                text: "2 results".into(),
                tone: StatusTone::Success,
            },
            results: ResultsView {
                items: vec![
                    sample_item("1", "Safari", "apps", "/Applications/Safari.app"),
                    sample_item(
                        "2",
                        "Safari Technology Preview",
                        "apps",
                        "/Applications/Safari Technology Preview.app",
                    ),
                ],
                selected_id: Some("1".into()),
                ..Default::default()
            },
            ..AppState::default()
        }
    }

    fn buffer_flat(buffer: &ratatui::buffer::Buffer) -> String {
        let mut out = String::with_capacity((buffer.area.width * buffer.area.height) as usize);
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                out.push(buffer[(x, y)].symbol().chars().next().unwrap_or(' '));
            }
            out.push('\n');
        }
        out
    }

    fn draw(state: &AppState, w: u16, h: u16) -> (String, ratatui::buffer::Buffer) {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal.draw(|f| render(f, state)).expect("draw");
        let buffer = terminal.backend().buffer().clone();
        (buffer_flat(&buffer), buffer)
    }

    #[test]
    fn render_search_80x24_smoke() {
        let (flat, _) = draw(&state_with_results(), 80, 24);
        assert!(flat.contains("Luma"), "brand title missing: {flat}");
        assert!(flat.contains("Safari"), "result title missing: {flat}");
        assert!(flat.contains("Apps"), "module label missing: {flat}");
        assert!(flat.contains("Launch"), "action hint missing: {flat}");
    }

    #[test]
    fn render_search_light_80x24() {
        let mut state = state_with_results();
        state.theme = Theme::resolve(ThemeMode::Light);
        let (flat, _) = draw(&state, 80, 24);
        assert!(flat.contains("Luma"));
        assert!(flat.contains("Safari"));
    }

    #[test]
    fn render_ascii_symbols_fallback() {
        let mut state = state_with_results();
        state.symbols = Symbols::ascii();
        let (flat, _) = draw(&state, 80, 24);
        assert!(flat.contains('>'), "ascii selected marker missing: {flat}");
        assert!(flat.contains("Ret"), "ascii enter hint missing: {flat}");
        assert!(!flat.contains('›'));
        assert!(!flat.contains('↵'));
    }

    #[test]
    fn render_match_highlight_requires_underline_on_query() {
        let state = state_with_results();
        let (_, buffer) = draw(&state, 80, 24);
        let mut found_underline = false;
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                let cell = &buffer[(x, y)];
                if matches!(cell.symbol(), "S" | "a" | "f")
                    && cell.modifier.contains(Modifier::UNDERLINED)
                    && cell.fg == Color::Cyan
                {
                    found_underline = true;
                }
            }
        }
        assert!(
            found_underline,
            "expected underlined cyan match cells for query 'saf'"
        );
    }

    #[test]
    fn render_kind_badge_permission_visible() {
        let state = AppState {
            theme: Theme::dark(),
            symbols: Symbols::unicode(),
            results: ResultsView {
                items: vec![sample_kind(
                    "p",
                    "Accessibility permission required",
                    "luma.clipboard",
                    "permission",
                    "Open System Settings",
                    "Open Settings",
                )],
                selected_id: Some("p".into()),
                ..Default::default()
            },
            ..AppState::default()
        };
        let (flat, _) = draw(&state, 80, 24);
        assert!(
            flat.contains("permission"),
            "permission badge missing: {flat}"
        );
    }

    #[test]
    fn render_kind_badge_warming_visible() {
        let state = AppState {
            theme: Theme::dark(),
            symbols: Symbols::unicode(),
            results: ResultsView {
                items: vec![sample_kind(
                    "w",
                    "App index warming",
                    "luma.apps",
                    "warming",
                    "cache refresh",
                    "Wait",
                )],
                selected_id: Some("w".into()),
                ..Default::default()
            },
            ..AppState::default()
        };
        let (flat, _) = draw(&state, 80, 24);
        assert!(flat.contains("warming"), "warming badge missing: {flat}");
    }

    #[test]
    fn render_kind_badge_unavailable_visible() {
        let state = AppState {
            theme: Theme::dark(),
            symbols: Symbols::unicode(),
            results: ResultsView {
                items: vec![sample_kind(
                    "u",
                    "Feature is unavailable",
                    "luma.example",
                    "unavailable",
                    "Not available locally",
                    "Details",
                )],
                selected_id: Some("u".into()),
                ..Default::default()
            },
            ..AppState::default()
        };
        let (flat, _) = draw(&state, 80, 24);
        assert!(
            flat.contains("unavailable"),
            "unavailable badge missing: {flat}"
        );
    }

    #[test]
    fn render_kind_badge_not_configured_visible() {
        let state = AppState {
            theme: Theme::dark(),
            symbols: Symbols::unicode(),
            results: ResultsView {
                items: vec![sample_kind(
                    "c",
                    "Choose a Notes root folder",
                    "luma.notes",
                    "onboarding",
                    "NotConfigured",
                    "Configure",
                )],
                selected_id: Some("c".into()),
                ..Default::default()
            },
            ..AppState::default()
        };
        let (flat, _) = draw(&state, 80, 24);
        assert!(
            flat.contains("not configured"),
            "not-configured badge missing: {flat}"
        );
    }

    #[test]
    fn render_search_120x40_scroll_cue() {
        let mut items = Vec::new();
        for i in 0..30 {
            items.push(sample_item(
                &format!("extra-{i}"),
                &format!("Safari Extra {i}"),
                "luma.apps",
                "/Applications/Extra.app",
            ));
        }
        let mut state = AppState {
            theme: Theme::dark(),
            symbols: Symbols::unicode(),
            prompt: "app saf".into(),
            term_width: 120,
            term_height: 40,
            results: ResultsView {
                selected_id: Some("extra-20".into()),
                items,
                ..Default::default()
            },
            ..AppState::default()
        };
        state.sync_results_viewport();
        state.results.ensure_selection_visible();
        let (flat, _) = draw(&state, 120, 40);
        assert!(
            flat.contains('↑') || flat.contains('↓'),
            "scroll cue missing: {flat}"
        );
    }

    #[test]
    fn truncate_uses_display_width_for_cjk() {
        let symbols = Symbols::unicode();
        let out = truncate("中文标题测试", 6, &symbols);
        assert!(display_width(&out) <= 6, "width overflow: {out}");
        assert!(out.contains('…') || out.ends_with('…'));
    }

    #[test]
    fn highlight_query_skips_module_trigger() {
        assert_eq!(highlight_query("app saf"), "saf");
        assert_eq!(highlight_query("safari"), "safari");
    }

    #[test]
    fn render_confirm_overlay_shows_target() {
        use crate::view_model::PendingAction;
        use luma_protocol::ActionDescriptorDto;

        let mut state = state_with_results();
        state.route = Route::ConfirmAction;
        state.pending_action = Some(PendingAction {
            result_id: "1".into(),
            action: ActionDescriptorDto {
                id: "quit".into(),
                label: "Force Quit".into(),
                risk: ActionRisk::Destructive,
                confirmation: true,
            },
        });
        let (flat, _) = draw(&state, 80, 24);
        assert!(flat.contains("DESTRUCTIVE") || flat.contains("Force Quit"));
        assert!(flat.contains("Safari"));
    }

    #[test]
    fn render_fatal_status_uses_error_color() {
        let mut state = state_with_results();
        state.status.set("Error: boom", StatusTone::Error);
        let (_, buffer) = draw(&state, 80, 24);
        let mut saw_error = false;
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                let cell = &buffer[(x, y)];
                if cell.symbol() == "E" && cell.fg == Color::Red {
                    saw_error = true;
                }
            }
        }
        assert!(saw_error, "expected red error status cells");
    }
}
