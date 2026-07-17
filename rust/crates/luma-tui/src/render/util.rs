use ratatui::style::Style;
use ratatui::text::Span;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::theme::Symbols;

pub(super) fn pad_line_to_width(spans: &mut Vec<Span<'static>>, width: usize, fill: Style) {
    let used: usize = spans
        .iter()
        .map(|s| display_width(s.content.as_ref()))
        .sum();
    if used < width {
        spans.push(Span::styled(" ".repeat(width - used), fill));
    }
}

pub(super) fn highlight_query(prompt: &str) -> String {
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
        "sec",
        "secret",
        "secrets",
        "fake",
        "echo",
        "p",
    ];
    let tokens: Vec<&str> = luma_domain::strip_command_prefix(prompt)
        .split_whitespace()
        .collect();
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

pub(super) fn highlighted_spans(
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

pub(super) fn display_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

pub(super) fn truncate(s: &str, max: usize, symbols: &Symbols) -> String {
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

pub(super) fn pad_right(s: &str, width: usize) -> String {
    let w = display_width(s);
    if w >= width {
        s.to_string()
    } else {
        format!("{s}{}", " ".repeat(width - w))
    }
}
