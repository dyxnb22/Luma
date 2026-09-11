//! Small UX helpers shared across modules (user-facing copy).

use luma_application::CommandSpec;
use luma_protocol::SearchItemDto;

pub(crate) fn command_spec(
    syntax: &str,
    description: &str,
    query: &str,
    example: Option<&str>,
) -> CommandSpec {
    let command = CommandSpec::new(syntax, description, query);
    match example {
        Some(example) => command.example(example),
        None => command,
    }
}

pub(crate) fn command_error(
    module_id: &str,
    id: &str,
    title: &str,
    message: impl Into<String>,
) -> SearchItemDto {
    SearchItemDto {
        id: id.into(),
        module_id: module_id.into(),
        title: title.into(),
        subtitle: Some(message.into()),
        kind: "command_error".into(),
        score: 100.0,
        primary_action_id: "noop".into(),
        primary_action_label: "Fix command".into(),
        ..Default::default()
    }
}

/// Map store/IO/sqlite noise into a short subtitle for daily use.
pub(crate) fn friendly_store_error(err: &str) -> String {
    let e = err.trim();
    let lower = e.to_ascii_lowercase();
    if lower.contains("readonly") || lower.contains("read-only") {
        return "Local database is read-only — check its folder permissions".into();
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
