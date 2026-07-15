//! Small UX helpers shared across modules (user-facing copy).

/// Map store/IO/sqlite noise into a short subtitle for daily use.
pub(crate) fn friendly_store_error(err: &str) -> String {
    let e = err.trim();
    let lower = e.to_ascii_lowercase();
    if lower.contains("readonly") || lower.contains("read-only") {
        return "Database locked — quit other Luma and retry".into();
    }
    if lower.contains("database is locked") || lower.contains("locked") {
        return "Database busy — quit other Luma and retry".into();
    }
    if lower.contains("sqlite") || e.starts_with("sqlite:") {
        return "Local database error — try restarting Luma".into();
    }
    if lower.contains("permission") || lower.contains("denied") {
        return "Permission denied for local store".into();
    }
    if e.chars().count() > 100 {
        let mut out: String = e.chars().take(97).collect();
        out.push('…');
        out
    } else if e.is_empty() {
        "Unavailable".into()
    } else {
        e.to_string()
    }
}
