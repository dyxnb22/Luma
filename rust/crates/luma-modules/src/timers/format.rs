use luma_application::TimerEntry;

pub(super) fn format_hms(ms: i64) -> String {
    let total_secs = (ms.max(0) / 1000) as u64;
    let h = total_secs / 3600;
    let m = (total_secs % 3600) / 60;
    let s = total_secs % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

pub(super) fn timer_title(entry: &TimerEntry, now_ms: i64) -> String {
    let glyph = match entry.state.as_str() {
        "running" => "▶",
        "paused" => "⏸",
        "completed" => "✓",
        _ => "○",
    };
    let time = if entry.kind == "countdown" {
        let left = entry.remaining_ms(now_ms).unwrap_or(0);
        if entry.state == "completed" {
            "done".into()
        } else {
            format!("{} left", format_hms(left))
        }
    } else {
        format_hms(entry.elapsed_ms(now_ms))
    };
    format!("{glyph} {}  {time}", entry.name)
}

pub(super) fn timer_subtitle(entry: &TimerEntry) -> String {
    let kind = if entry.kind == "countdown" {
        match entry.duration_ms {
            Some(ms) => format!("countdown · {}", format_hms(ms)),
            None => "countdown".into(),
        }
    } else {
        "stopwatch".into()
    };
    format!("{kind} · {}", entry.state)
}

pub(super) fn primary_for(entry: &TimerEntry) -> (&'static str, &'static str) {
    match entry.state.as_str() {
        "running" => ("pause", "Pause"),
        "paused" => ("resume", "Resume"),
        "completed" => ("reset", "Reset"),
        _ => ("start", "Start"),
    }
}
