pub(super) const DEFAULT_POMO_MINUTES: u32 = 25;
pub(super) const MAX_COUNTDOWN_MINUTES: u32 = 24 * 60;

pub(super) fn parse_minutes_token(token: &str) -> Option<u32> {
    let n: u32 = token.parse().ok()?;
    if (1..=MAX_COUNTDOWN_MINUTES).contains(&n) {
        Some(n)
    } else {
        None
    }
}

/// Parse `pomo|cd|countdown [minutes] [name…]` or bare `NN [name…]`.
pub(super) fn parse_countdown_spec(rest: &str) -> Option<(u32, String)> {
    let rest = rest.trim();
    if rest.is_empty() {
        return Some((DEFAULT_POMO_MINUTES, "Pomodoro".into()));
    }
    let mut parts = rest.split_whitespace();
    let first = parts.next()?;
    let (minutes, name_bits): (u32, Vec<&str>) = if let Some(m) = parse_minutes_token(first) {
        (m, parts.collect())
    } else if matches!(first, "pomo" | "pomodoro" | "cd" | "countdown") {
        match parts.next() {
            Some(t) => {
                if let Some(m) = parse_minutes_token(t) {
                    (m, parts.collect())
                } else {
                    // `/tm pomo deep work` — default minutes, name starts at first
                    let mut name = vec![t];
                    name.extend(parts);
                    (DEFAULT_POMO_MINUTES, name)
                }
            }
            None => (DEFAULT_POMO_MINUTES, Vec::new()),
        }
    } else {
        return None;
    };
    let name = if name_bits.is_empty() {
        if minutes == DEFAULT_POMO_MINUTES {
            "Pomodoro".into()
        } else {
            format!("{minutes}m")
        }
    } else {
        name_bits.join(" ")
    };
    Some((minutes, name))
}

pub(super) fn parse_stopwatch_name(rest: &str) -> String {
    let rest = rest.trim();
    let stripped = rest
        .strip_prefix("sw ")
        .or_else(|| rest.strip_prefix("stopwatch "))
        .or_else(|| rest.strip_prefix("start "))
        .unwrap_or(rest)
        .trim();
    if stripped.is_empty()
        || matches!(
            stripped,
            "sw" | "stopwatch" | "start" | "pomo" | "pomodoro" | "cd" | "countdown"
        )
    {
        "Stopwatch".into()
    } else {
        stripped.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_countdown_variants() {
        assert_eq!(parse_countdown_spec(""), Some((25, "Pomodoro".into())));
        assert_eq!(parse_countdown_spec("pomo"), Some((25, "Pomodoro".into())));
        assert_eq!(parse_countdown_spec("pomo 50"), Some((50, "50m".into())));
        assert_eq!(
            parse_countdown_spec("25 deep work"),
            Some((25, "deep work".into()))
        );
        assert_eq!(
            parse_countdown_spec("pomo deep work"),
            Some((25, "deep work".into()))
        );
    }
}
