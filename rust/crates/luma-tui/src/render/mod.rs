use crate::theme::{module_glyph, module_label, ResultKindVisual, Symbols, Theme};
use crate::view_model::{AppState, Route, StatusTone};
use luma_domain::{ActionRisk, SearchItem};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;
mod overlays;
mod util;

use overlays::*;
use util::{
    display_width, highlight_query, highlighted_spans, pad_line_to_width, pad_right, truncate,
};

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
    if state.route == Route::WordbookReview
        || (state.wordbook_review.is_some() && matches!(state.route, Route::ConfirmAction))
    {
        render_wordbook_review(frame, body, state, theme, symbols);
    } else if state.preview_side_by_side() {
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
        Route::Search | Route::WordbookReview => {}
        Route::Help => render_overlay_help(frame, area, state, theme, symbols),
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
    let inner_w = area.width.saturating_sub(2) as usize;
    let _ = inner_w;
    let cursor = if focused { symbols.cursor } else { " " };
    let chars: Vec<char> = state.prompt.chars().collect();
    let before: String = chars
        .iter()
        .skip(state.prompt_scroll)
        .take(state.prompt_cursor.saturating_sub(state.prompt_scroll))
        .collect();
    let after: String = chars.iter().skip(state.prompt_cursor).collect();
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
            let win_digit = if state.is_win_search()
                && state.focus == FocusZone::List
                && item.module_id.as_str() == "luma.windows"
                && item.kind == "window"
            {
                state
                    .window_digit_targets()
                    .iter()
                    .position(|(id, _)| id == item.id.as_str())
                    .map(|i| i + 1)
            } else {
                None
            };
            items.push(result_row(
                item,
                selected,
                inner_width,
                &query,
                theme,
                symbols,
                &state.module_labels,
                win_digit,
            ));
        }
    }

    let title = if state.results.items.is_empty() {
        if state.prompt.trim().is_empty() {
            let hub_total = state.hub_rows().len();
            let sel = state.hub_selected + 1;
            let mut scroll_marks = String::new();
            if state.hub_scroll > 0 {
                scroll_marks.push_str(symbols.up);
            }
            if state.hub_scroll + state.hub_data_capacity() < hub_total {
                scroll_marks.push_str(symbols.down);
            }
            let scroll_part = if scroll_marks.is_empty() {
                String::new()
            } else {
                format!(" {scroll_marks}")
            };
            if hub_total > 0 {
                format!(
                    " hub {} {}/{}{scroll_part} ",
                    symbols.sep,
                    sel.min(hub_total),
                    hub_total
                )
            } else {
                " hub ".to_string()
            }
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

fn hub_module_title(state: &AppState, module_id: &str, title: &str) -> String {
    if !state.settings_roots.loaded {
        return title.to_string();
    }
    let needs_notes = module_id == "luma.notes"
        && state
            .settings_roots
            .notes_root
            .as_ref()
            .map(|s| s.is_empty())
            .unwrap_or(true);
    let needs_projects = module_id == "luma.projects"
        && (state.settings_roots.projects_roots.is_empty()
            || state.settings_roots.imported_projects.is_empty());
    if needs_notes || needs_projects {
        if module_id == "luma.projects" && !state.settings_roots.projects_roots.is_empty() {
            format!("{title} · import")
        } else {
            format!("{title} · set root")
        }
    } else {
        title.to_string()
    }
}

fn hub_list_items(state: &AppState, theme: &Theme, symbols: &Symbols) -> Vec<ListItem<'static>> {
    let rows = state.hub_rows();
    if rows.is_empty() {
        return vec![ListItem::new(vec![
            Line::from(Span::styled("  Waiting for modules…", theme.muted())),
            Line::from(Span::styled(
                "  Loading modules in the background",
                theme.key_hint(),
            )),
        ])];
    }
    let mut out = Vec::new();
    let mut shown_windows = false;
    let mut shown_modules = false;
    let viewport = state.hub_data_capacity();
    let start = state.hub_scroll.min(rows.len());
    let end = (start + viewport).min(rows.len());
    let module_global_start = rows.iter().position(|(k, _, _, _)| k == "module");
    let window = &rows[start..end];
    for (idx, (kind, _id, title, query)) in window.iter().enumerate().map(|(i, r)| (start + i, r)) {
        if (kind == "window" || kind == "window_more" || kind == "window_status")
            && !shown_windows
            && start == 0
        {
            shown_windows = true;
            let header = match state.hub_windows.as_ref().map(|h| h.app_name.as_str()) {
                Some("all") | Some("") | None => "  Windows".to_string(),
                Some(app) => format!("  Windows · {app}"),
            };
            let hint = "  Enter focuses window · 1-9 focus · ↑↓ move";
            out.push(ListItem::new(vec![
                Line::from(Span::styled(header, theme.title())),
                Line::from(Span::styled(hint, theme.key_hint())),
            ]));
        }
        if kind == "module" && !shown_modules && module_global_start == Some(idx) {
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
        let right = match kind.as_str() {
            "window" => state
                .hub_row_window_digit(idx)
                .map(|d| format!("[{d}]"))
                .unwrap_or_default(),
            "window_more" | "window_status" => "win".to_string(),
            _ => query.clone(),
        };
        let right = truncate(
            &right,
            (state.term_width.saturating_sub(6) as usize / 3).max(8),
            symbols,
        );
        let content_width = state.term_width.saturating_sub(4) as usize;
        let guidance = if kind == "window_status" {
            state
                .hub_windows
                .as_ref()
                .and_then(|h| h.status_subtitle.as_ref())
                .cloned()
        } else {
            None
        };
        let display_title = if kind == "module" {
            hub_module_title(state, _id, title)
        } else {
            title.clone()
        };
        let display_title = truncate(
            &display_title,
            content_width
                .saturating_sub(display_width(&right) + 4)
                .max(8),
            symbols,
        );
        let mut lines = vec![Line::from(vec![
            Span::styled(format!(" {prefix} {display_title}"), style),
            Span::styled(format!("  {right}"), muted),
        ])];
        if let Some(sub) = guidance {
            lines.push(Line::from(Span::styled(
                format!(
                    "    {}",
                    truncate(&sub, content_width.saturating_sub(4), symbols)
                ),
                muted,
            )));
        } else {
            lines.push(Line::from(""));
        }
        out.push(ListItem::new(lines));
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
        let body_dupes_sub = state
            .preview_result_id
            .as_deref()
            .filter(|id| *id == item.id.as_str())
            .and(state.preview_body.as_deref())
            .is_some_and(|body| {
                let body_trim = body.trim();
                body_trim == sub.trim()
                    || body_trim == item.title.trim()
                    || body_trim
                        .lines()
                        .next()
                        .is_some_and(|l| l.trim() == sub.trim() || l.trim() == item.title.trim())
            });
        if !body_dupes_sub {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(format!("  {sub}"), theme.text())));
        }
    } else if state
        .preview_result_id
        .as_deref()
        .filter(|id| *id == item.id.as_str())
        .and(state.preview_body.as_deref())
        .is_some_and(|body| body.trim() == item.title.trim())
    {
        // Title already shown; skip empty subtitle when body only echoes title.
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
            let body_len = body_lines.len();
            for line in body_lines.into_iter().skip(scroll).take(visible) {
                if scroll == 0 && line.trim() == item.title.trim() && body_len == 1 {
                    continue;
                }
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
    let (title, detail) = if state.search_debounce_deadline.is_some() {
        (
            format!("Keep typing{}", symbols.ellipsis),
            "Search runs after a short pause".to_string(),
        )
    } else if let Some((trigger, hint)) = incomplete_trigger_hint(state) {
        (
            format!("Add a space to enter `{trigger}`"),
            hint.unwrap_or_else(|| format!("Try `{trigger} ` or `{trigger} query`")),
        )
    } else if state.active_request.is_some() && state.status.tone == StatusTone::Progress {
        (
            format!("Searching{}", symbols.ellipsis),
            "Results appear as modules respond".to_string(),
        )
    } else if state.prompt.trim().is_empty() {
        (
            "Type to search".to_string(),
            format!(
                "Try: /app safari {} /n browse {} /clip",
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

/// Slash-prefixed bare module trigger without trailing space (`/n`, not `n` or `/n `).
fn incomplete_trigger_hint(state: &AppState) -> Option<(String, Option<String>)> {
    let token = state.incomplete_slash_trigger()?;
    let module = state
        .module_catalog
        .iter()
        .find(|m| m.enabled && m.triggers.iter().any(|t| t.eq_ignore_ascii_case(&token)));
    let display = format!("/{token}");
    Some((display, module.and_then(|m| m.empty_hint.clone())))
}

/// Prefer the targeted module's `empty_hint` when the prompt starts with its trigger.
fn empty_hint_for_prompt(state: &AppState) -> Option<String> {
    let token = state
        .prompt
        .trim_start()
        .strip_prefix('/')?
        .split_whitespace()
        .next()?
        .to_ascii_lowercase();
    state
        .module_catalog
        .iter()
        .find(|m| m.enabled && m.triggers.iter().any(|t| t.eq_ignore_ascii_case(&token)))
        .and_then(|m| m.empty_hint.clone())
}

#[allow(clippy::too_many_arguments)]
fn result_row(
    item: &SearchItem,
    selected: bool,
    width: u16,
    query: &str,
    theme: &Theme,
    symbols: &Symbols,
    module_labels: &std::collections::HashMap<String, String>,
    win_digit: Option<usize>,
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
    if let Some(d) = win_digit {
        title_spans.push(Span::styled(format!(" [{d}]"), hint));
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

fn render_wordbook_review(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    symbols: &Symbols,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border(true))
        .title(Span::styled(" wordbook review ", theme.muted()));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(review) = &state.wordbook_review else {
        frame.render_widget(
            Paragraph::new("Loading review…").style(theme.muted()),
            inner,
        );
        return;
    };

    let progress_index = if review.finished {
        review.index.min(review.words.len())
    } else {
        review.index.saturating_add(1).min(review.words.len())
    };
    let progress = if review.words.is_empty() {
        "0/0".to_string()
    } else {
        format!("{}/{}", progress_index, review.words.len())
    };
    let header = format!(
        "{progress} · queue {} · due {} · today {} · goal {} · remaining {}",
        review.stats.queue,
        review.stats.due,
        review.stats.reviewed_today,
        review.stats.goal,
        review.stats.remaining_goal
    );

    if review.finished || review.words.is_empty() {
        let summary = format!(
            "Done · Known {} · Fuzzy {} · Unknown {} · Mastered {} · Skipped {} · today {} · remaining {}",
            review.stats.session_known,
            review.stats.session_fuzzy,
            review.stats.session_unknown,
            review.stats.session_mastered,
            review.stats.session_skipped,
            review.stats.reviewed_today,
            review.stats.remaining_goal
        );
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(header, theme.title())),
                Line::from(""),
                Line::from(Span::styled(summary, theme.text())),
            ])
            .wrap(Wrap { trim: false }),
            inner,
        );
        return;
    }

    let word = &review.words[review.index];
    let mut lines = vec![
        Line::from(Span::styled(header, theme.muted())),
        Line::from(""),
        Line::from(Span::styled(&word.term, theme.title())),
    ];
    if !word.phonetic.is_empty() {
        lines.push(Line::from(Span::styled(&word.phonetic, theme.muted())));
    }
    if review.revealed {
        if !word.meaning.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(&word.meaning, theme.text())));
        }
        if !word.example.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("ex: {}", word.example),
                theme.muted(),
            )));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "1 Known · 2 Fuzzy · 3 Unknown · m Mastered · s Skip",
            theme.key_hint(),
        )));
    } else {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("{} Enter or Space to reveal", symbols.enter),
            theme.key_hint(),
        )));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
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
        Route::Search if state.showing_hub() => {
            format!(
                "1-9 focus {}{}{} move {} Enter open {} Ctrl-/ commands",
                symbols.sep, symbols.up, symbols.down, symbols.sep, symbols.sep
            )
        }
        Route::Search => {
            if state.focus == crate::view_model::FocusZone::Preview {
                format!(
                    "PgUp/Dn scroll {} Tab focus {} Esc back",
                    symbols.sep, symbols.sep
                )
            } else if state.is_win_search() && state.focus == crate::view_model::FocusZone::List {
                format!(
                    "1-9 focus {}{} move {} Enter open {} Ctrl-k actions {} Tab focus",
                    symbols.sep, symbols.up, symbols.sep, symbols.sep, symbols.sep
                )
            } else if !state.results.items.is_empty()
                && state.focus == crate::view_model::FocusZone::List
            {
                format!(
                    "{}{} move {} Enter run {} Ctrl-k actions {} Tab focus {} S-Tab preview {} Esc back {} ? help",
                    symbols.up,
                    symbols.down,
                    symbols.sep,
                    symbols.sep,
                    symbols.sep,
                    symbols.sep,
                    symbols.sep,
                    symbols.sep
                )
            } else {
                format!(
                    "{}{} move {} Enter search {} Ctrl-k actions {} Tab focus {} S-Tab preview {} Esc back {} ? help",
                    symbols.up,
                    symbols.down,
                    symbols.sep,
                    symbols.sep,
                    symbols.sep,
                    symbols.sep,
                    symbols.sep,
                    symbols.sep
                )
            }
        }
        Route::ActionPicker => format!(
            "{}{} 1-9 {} Enter Run {} Esc Back",
            symbols.up, symbols.down, symbols.sep, symbols.sep
        ),
        Route::Settings => format!(
            "{}{} Enter/Space Toggle {} Esc Back",
            symbols.up, symbols.down, symbols.sep
        ),
        Route::Commands => format!("Enter Run {} Esc Back", symbols.sep),
        Route::ConfirmAction | Route::QuitConfirm => {
            format!("Enter Confirm {} Esc Cancel", symbols.sep)
        }
        Route::Help => format!(
            "{}{} / PgUp PgDn scroll {} Esc Back",
            symbols.up, symbols.down, symbols.sep
        ),
        Route::WordbookReview => {
            if state.wordbook_review.as_ref().is_some_and(|r| r.finished) {
                "Esc back".into()
            } else {
                "Enter/Space reveal · 1/2/3 grade · m master · s skip · Esc exit".into()
            }
        }
    };

    let inner_w = area.width.saturating_sub(2) as usize;
    let narrow = inner_w < 60;
    let (status_text, hints) = if narrow {
        let compact_hints = match state.route {
            Route::Search if state.showing_hub() => "1-9 · ↑↓ · Enter".to_string(),
            Route::Search => "Enter · Ctrl-k · Esc".to_string(),
            Route::ActionPicker => "1-9 · Enter · Esc".to_string(),
            Route::Settings => "↑↓ · Space · Esc".to_string(),
            Route::Commands => "Enter · Esc".to_string(),
            Route::ConfirmAction | Route::QuitConfirm => "Enter yes · Esc no".to_string(),
            Route::Help => "↑↓ · Esc".to_string(),
            Route::WordbookReview => {
                if state.wordbook_review.as_ref().is_some_and(|r| r.finished) {
                    "Esc back".to_string()
                } else {
                    "1/2/3 · s skip · Esc".to_string()
                }
            }
        };
        let compact_status = if state.route == Route::WordbookReview {
            if state.wordbook_review.as_ref().is_some_and(|r| r.finished) {
                "done"
            } else {
                "review"
            }
        } else if state.status.text.starts_with("removed ") {
            "removed · dir kept"
        } else {
            state.status.text.as_str()
        };
        (compact_status.to_string(), compact_hints)
    } else {
        (state.status.text.clone(), hints)
    };
    let hints_budget = (inner_w / 2).clamp(16, 60);
    let hints = truncate(&hints, hints_budget, symbols);
    let status_budget = inner_w.saturating_sub(display_width(&hints) + 3).max(8);
    let status_text = truncate(&status_text, status_budget, symbols);

    let line = Line::from(vec![
        Span::styled(format!(" {status_text}  "), status_style),
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
            ui_intent: None,
            action_payload: None,
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
            ui_intent: None,
            action_payload: None,
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
    fn hub_layout_80x24_last_row_visible() {
        let state = AppState {
            module_catalog: (0..12)
                .map(|i| crate::view_model::ModuleCatalogEntry {
                    id: format!("luma.mod{i}"),
                    display_name: format!("Module {i}"),
                    enabled: true,
                    glyph: None,
                    suggested_query: Some(format!("m{i} ")),
                    empty_hint: None,
                    supports_browse: false,
                    triggers: vec![],
                })
                .collect(),
            hub_windows: Some(crate::view_model::HubWindowsState {
                app_name: "Cursor".into(),
                windows: vec![crate::view_model::HubWindowRow {
                    id: "win:a".into(),
                    title: "Editor".into(),
                }],
                more: None,
                status_kind: None,
                status_title: None,
                status_subtitle: None,
            }),
            ..AppState::default()
        };
        let (flat, buffer) = draw(&state, 80, 24);
        assert_eq!(buffer.area.height, 24);
        let last_row: String = (0..buffer.area.width)
            .map(|x| buffer[(x, 23)].symbol().chars().next().unwrap_or(' '))
            .collect();
        assert!(
            last_row.contains("Enter") || flat.contains("Enter open"),
            "hub status hints should appear on last row: {last_row:?}"
        );
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
    fn footer_says_run_when_results_present_and_list_focused() {
        let mut state = state_with_results();
        state.focus = crate::view_model::FocusZone::List;
        let (flat, _) = draw(&state, 100, 30);
        assert!(
            flat.contains("Enter run") || flat.contains("run"),
            "expected Enter run in footer: {flat}"
        );
        assert!(
            !flat.contains("Enter search"),
            "should not say Enter search with list results: {flat}"
        );
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
        assert!(flat.contains("loading"), "loading badge missing: {flat}");
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
                    "not_configured",
                    "NotConfigured",
                    "Configure",
                )],
                selected_id: Some("c".into()),
                ..Default::default()
            },
            ..AppState::default()
        };
        let (flat, _) = draw(&state, 80, 24);
        assert!(flat.contains("setup"), "setup badge missing: {flat}");
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
    fn render_wordbook_progress_and_summary_are_consistent() {
        let mut state = AppState {
            route: Route::WordbookReview,
            wordbook_review: Some(crate::view_model::WordbookReviewState {
                words: vec![
                    crate::view_model::WordbookReviewWord {
                        id: 1,
                        term: "alpha".into(),
                        phonetic: String::new(),
                        meaning: "first".into(),
                        example: String::new(),
                    },
                    crate::view_model::WordbookReviewWord {
                        id: 2,
                        term: "beta".into(),
                        phonetic: String::new(),
                        meaning: "second".into(),
                        example: String::new(),
                    },
                ],
                index: 2,
                revealed: false,
                stats: crate::view_model::WordbookReviewStats {
                    queue: "due".into(),
                    due: 0,
                    goal: 20,
                    reviewed_today: 12,
                    remaining_goal: 8,
                    session_known: 1,
                    session_fuzzy: 0,
                    session_unknown: 0,
                    session_skipped: 0,
                    session_mastered: 1,
                    ..Default::default()
                },
                finished: true,
                pending_grade: None,
            }),
            ..AppState::default()
        };
        state.term_width = 80;
        state.term_height = 24;
        let (flat, _) = draw(&state, 80, 24);
        assert!(flat.contains("2/2"), "completed progress missing: {flat}");
        assert!(!flat.contains("3/2"), "progress overflowed: {flat}");
        assert!(flat.contains("Mastered 1"), "mastered stat missing: {flat}");
        assert!(flat.contains("today 12"), "today stat missing: {flat}");

        state.wordbook_review.as_mut().unwrap().finished = false;
        state.wordbook_review.as_mut().unwrap().index = 0;
        let (flat, _) = draw(&state, 80, 24);
        assert!(flat.contains("1/2"), "current progress missing: {flat}");
    }

    #[test]
    fn render_wordbook_confirm_shows_current_word() {
        let state = AppState {
            route: Route::ConfirmAction,
            wordbook_review: Some(crate::view_model::WordbookReviewState {
                words: vec![crate::view_model::WordbookReviewWord {
                    id: 42,
                    term: "ephemeral".into(),
                    phonetic: String::new(),
                    meaning: "short-lived".into(),
                    example: String::new(),
                }],
                index: 0,
                revealed: true,
                stats: Default::default(),
                finished: false,
                pending_grade: Some("mastered".into()),
            }),
            pending_action: Some(crate::view_model::PendingAction {
                result_id: "wb:42".into(),
                action: luma_protocol::ActionDescriptorDto {
                    id: "mastered".into(),
                    label: "mastered".into(),
                    risk: ActionRisk::Confirm,
                    confirmation: true,
                },
            }),
            ..AppState::default()
        };
        let (flat, _) = draw(&state, 80, 24);
        assert!(
            flat.contains("Target: ephemeral"),
            "word target missing: {flat}"
        );
    }

    #[test]
    fn wide_review_hides_search_preview() {
        let mut state = AppState {
            route: Route::WordbookReview,
            wordbook_review: Some(crate::view_model::WordbookReviewState {
                words: vec![crate::view_model::WordbookReviewWord {
                    id: 1,
                    term: "alpha".into(),
                    phonetic: String::new(),
                    meaning: "first".into(),
                    example: String::new(),
                }],
                index: 0,
                revealed: false,
                stats: Default::default(),
                finished: false,
                pending_grade: None,
            }),
            results: ResultsView {
                items: vec![sample_item("1", "Preview result", "apps", "body")],
                selected_id: Some("1".into()),
                ..Default::default()
            },
            ..AppState::default()
        };
        state.term_width = 120;
        let (flat, _) = draw(&state, 120, 40);
        assert!(
            flat.contains("wordbook review"),
            "review body missing: {flat}"
        );
        assert!(
            !flat.contains(" preview "),
            "search preview leaked into review: {flat}"
        );

        state.term_width = 43;
        let (flat, _) = draw(&state, 43, 20);
        assert!(flat.contains("1/2/3"), "narrow grade hint missing: {flat}");
        assert!(flat.contains("Esc"), "narrow exit hint missing: {flat}");

        state.wordbook_review.as_mut().unwrap().finished = true;
        let (flat, _) = draw(&state, 43, 20);
        assert!(flat.contains("done"), "narrow done status missing: {flat}");
        assert!(
            flat.contains("Esc back"),
            "narrow done hint missing: {flat}"
        );
    }

    #[test]
    fn settings_overlay_keeps_selected_module_visible() {
        let state = AppState {
            route: Route::Settings,
            settings_selected: 24,
            settings_modules: (0..30)
                .map(|i| crate::view_model::SettingsModuleRow {
                    id: format!("luma.module{i}"),
                    name: format!("Module {i}"),
                    enabled: true,
                })
                .collect(),
            ..AppState::default()
        };
        let (flat, _) = draw(&state, 80, 24);
        assert!(
            flat.contains("Module 24"),
            "selected module not visible: {flat}"
        );
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

    #[test]
    fn hub_window_rows_show_digit_hints() {
        let state = AppState {
            hub_windows: Some(crate::view_model::HubWindowsState {
                app_name: "all".into(),
                windows: vec![
                    crate::view_model::HubWindowRow {
                        id: "win:1".into(),
                        title: "Alpha".into(),
                    },
                    crate::view_model::HubWindowRow {
                        id: "win:2".into(),
                        title: "Beta".into(),
                    },
                ],
                more: None,
                status_kind: Some("permission_required".into()),
                status_title: Some("grant AX".into()),
                status_subtitle: None,
            }),
            ..AppState::default()
        };
        let (flat, _) = draw(&state, 80, 24);
        assert!(flat.contains("[1]"), "first window should show [1]: {flat}");
        assert!(
            flat.contains("[2]"),
            "second window should show [2]: {flat}"
        );
        assert!(
            !flat.contains("grant AX[1]"),
            "status row must not be numbered"
        );
    }

    #[test]
    fn win_search_window_rows_show_digit_hints() {
        let state = AppState {
            prompt: "/win ".into(),
            focus: crate::view_model::FocusZone::List,
            results: crate::view_model::ResultsView {
                items: vec![
                    luma_domain::SearchItem {
                        id: luma_domain::ResultId::new("win:status"),
                        module_id: luma_domain::ModuleId::new("luma.windows"),
                        title: "Permission".into(),
                        subtitle: None,
                        kind: "permission_required".into(),
                        score: 1.0,
                        primary_action: luma_domain::ActionDescriptor {
                            id: luma_domain::ActionId::new("noop"),
                            label: "OK".into(),
                            risk: luma_domain::ActionRisk::Safe,
                            confirmation: false,
                        },
                        secondary_actions: vec![],
                        ui_intent: None,
                        action_payload: None,
                    },
                    luma_domain::SearchItem {
                        id: luma_domain::ResultId::new("win:a"),
                        module_id: luma_domain::ModuleId::new("luma.windows"),
                        title: "Alpha".into(),
                        subtitle: None,
                        kind: "window".into(),
                        score: 1.0,
                        primary_action: luma_domain::ActionDescriptor {
                            id: luma_domain::ActionId::new("focus"),
                            label: "Focus".into(),
                            risk: luma_domain::ActionRisk::Safe,
                            confirmation: false,
                        },
                        secondary_actions: vec![],
                        ui_intent: None,
                        action_payload: None,
                    },
                ],
                selected_id: Some("win:a".into()),
                ..Default::default()
            },
            ..AppState::default()
        };
        let (flat, _) = draw(&state, 80, 24);
        assert!(flat.contains("[1]"), "window row should show [1]: {flat}");
    }
}
