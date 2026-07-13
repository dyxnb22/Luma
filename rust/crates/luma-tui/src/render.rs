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

    render_prompt(
        frame,
        chunks[0],
        state,
        theme,
        symbols,
        matches!(state.route, Route::Search),
    );
    render_results(frame, chunks[1], state, theme, symbols);
    render_status(frame, chunks[2], state, theme, symbols);

    match state.route {
        Route::Search => {}
        Route::Help => render_overlay_help(frame, area, theme, symbols),
        Route::Doctor => render_overlay_doctor(frame, area, state, theme, symbols),
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
    let line = Line::from(vec![
        Span::styled("  ", theme.muted()),
        Span::styled(state.prompt.as_str(), theme.text()),
        Span::styled(cursor, theme.accent()),
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
    let inner_height = area.height.saturating_sub(2) as usize;
    let inner_width = area.width.saturating_sub(2);
    let selected_idx = state
        .results
        .selected_id
        .as_ref()
        .and_then(|id| {
            state
                .results
                .items
                .iter()
                .position(|i| i.id.as_str() == id.as_str())
        })
        .unwrap_or(0);

    let rows_capacity = inner_height / 2;
    let total = state.results.items.len();
    let scroll = if rows_capacity == 0 || total == 0 {
        0
    } else {
        selected_idx.saturating_sub(rows_capacity.saturating_sub(1))
    };
    let visible_count = if total == 0 {
        0
    } else {
        rows_capacity.max(1).min(total.saturating_sub(scroll))
    };
    let has_above = scroll > 0;
    let has_below = scroll + visible_count < total;

    let mut items: Vec<ListItem> = Vec::new();
    if state.results.items.is_empty() {
        items.push(empty_state_item(state, theme, symbols));
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
            ));
        }
    }

    let title = if state.results.items.is_empty() {
        " results ".to_string()
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
            .border_style(theme.border(matches!(state.route, Route::Search)))
            .title(Span::styled(title, theme.muted())),
    );
    frame.render_widget(list, area);
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

fn result_row(
    item: &SearchItem,
    selected: bool,
    width: u16,
    query: &str,
    theme: &Theme,
    symbols: &Symbols,
) -> ListItem<'static> {
    let kind = ResultKindVisual::from_kind(&item.kind);
    let glyph = module_glyph(item.module_id.as_str());
    let module = module_label(item.module_id.as_str()).to_string();
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
        "s",
        "snip",
        "tr",
        "translate",
        "t",
        "todo",
        "proj",
        "kill",
        "media",
        "word",
        "wordbook",
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
        Route::Search => format!(
            "Tab Actions {} Esc Clear {} ? Help",
            symbols.sep, symbols.sep
        ),
        Route::ActionPicker => format!("Enter Run {} Esc Back", symbols.sep),
        Route::ConfirmAction | Route::QuitConfirm => {
            format!("Enter Confirm {} Esc Cancel", symbols.sep)
        }
        Route::Help | Route::Doctor => "Esc Back".to_string(),
    };
    let count = if state.results.items.is_empty() {
        String::new()
    } else {
        let mut seen = Vec::new();
        for item in &state.results.items {
            let label = module_label(item.module_id.as_str());
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
    let overlay = overlay_area(area, 14);
    frame.render_widget(Clear, overlay);
    let text = [
        "Type to search".to_string(),
        format!("{}{}  move selection", symbols.up, symbols.down),
        "Enter  primary action".to_string(),
        "Tab  action list".to_string(),
        "Esc  cancel / back / clear".to_string(),
        "?  help".to_string(),
        ":doctor  diagnostics".to_string(),
        "Ctrl-C  quit prompt · Enter confirm".to_string(),
        String::new(),
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
    let overlay = overlay_area(area, 12);
    frame.render_widget(Clear, overlay);
    let text = state.doctor_diagnostic.as_ref().map_or_else(
        || format!("Waiting for engine diagnostics{}", symbols.ellipsis),
        |diagnostic| diagnostic.to_string(),
    );
    let widget = Paragraph::new(text).wrap(Wrap { trim: false }).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border(true))
            .title(Span::styled(" doctor ", theme.title())),
    );
    frame.render_widget(widget, overlay);
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
                    "Reminders permission required",
                    "luma.todo",
                    "permission",
                    "Open System Settings",
                    "Open Settings",
                )],
                selected_id: Some("p".into()),
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
                    "Wordbook is unavailable",
                    "luma.wordbook",
                    "unavailable",
                    "Storage pending",
                    "Details",
                )],
                selected_id: Some("u".into()),
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
        let state = AppState {
            theme: Theme::dark(),
            symbols: Symbols::unicode(),
            prompt: "app saf".into(),
            results: ResultsView {
                selected_id: Some("extra-20".into()),
                items,
            },
            ..AppState::default()
        };
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
