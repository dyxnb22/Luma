use crate::theme::{Symbols, Theme};
use crate::view_model::AppState;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Wrap};
use ratatui::Frame;

pub(super) fn render_wordbook_review(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    symbols: &Symbols,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(symbols.border_type())
        .border_style(theme.border(true))
        .style(theme.surface())
        .padding(Padding::horizontal(1))
        .title(Span::styled(" WORDBOOK REVIEW ", theme.panel_title(true)));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(review) = &state.wordbook.review else {
        frame.render_widget(
            Paragraph::new("Loading review…")
                .style(theme.muted())
                .block(Block::default().style(theme.surface())),
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
                Line::from(Span::styled(
                    summary,
                    theme.text().add_modifier(Modifier::BOLD),
                )),
            ])
            .style(theme.surface())
            .wrap(Wrap { trim: false }),
            inner,
        );
        return;
    }

    let word = &review.words[review.index];
    let mut lines = vec![
        Line::from(Span::styled(header, theme.accent_secondary())),
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
        lines.push(Line::from(vec![
            Span::styled("1", theme.keycap()),
            Span::styled(" Known  ", theme.muted()),
            Span::styled("2", theme.keycap()),
            Span::styled(" Fuzzy  ", theme.muted()),
            Span::styled("3", theme.keycap()),
            Span::styled(" Unknown  ", theme.muted()),
            Span::styled("m", theme.keycap()),
            Span::styled(" Mastered  ", theme.muted()),
            Span::styled("s", theme.keycap()),
            Span::styled(" Skip", theme.muted()),
        ]));
    } else {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(format!("{} Enter", symbols.enter), theme.keycap()),
            Span::styled(" or ", theme.muted()),
            Span::styled("Space", theme.keycap()),
            Span::styled(" to reveal", theme.muted()),
        ]));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .style(theme.surface())
            .wrap(Wrap { trim: false }),
        inner,
    );
}
