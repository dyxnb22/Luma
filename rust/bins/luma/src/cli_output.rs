//! CLI exit codes and JSON outcome mapping for non-interactive commands.

use luma_protocol::ActionOutcomeDto;

/// Exit code for a one-shot action outcome.
pub fn action_exit_code(outcome: &ActionOutcomeDto) -> i32 {
    match outcome {
        ActionOutcomeDto::Success { .. } => 0,
        ActionOutcomeDto::Failed { .. } => 1,
        ActionOutcomeDto::Cancelled => 2,
    }
}

/// Actionable doctor summary for CLI (default non-JSON output).
/// Keep in sync with `luma_tui::render::overlays::format_doctor_summary`.
pub fn format_doctor_summary(diagnostic: &serde_json::Value) -> String {
    let mut lines = Vec::new();
    if let Some(remediation) = diagnostic.get("remediation").and_then(|v| v.as_array()) {
        let tips: Vec<&str> = remediation
            .iter()
            .filter_map(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .collect();
        if !tips.is_empty() {
            lines.push("Next steps:".to_string());
            for tip in tips {
                lines.push(format!("  · {tip}"));
            }
            lines.push(String::new());
        }
    }
    if let Some(ax) = diagnostic.get("accessibility") {
        lines.push("Accessibility:".to_string());
        let trusted = ax.get("trusted").and_then(|v| v.as_bool()).unwrap_or(false);
        lines.push(format!("  trusted: {trusted}"));
        if let Some(guidance) = ax.get("guidance").and_then(|v| v.as_str()) {
            lines.push(format!("  {guidance}"));
        }
        lines.push(String::new());
    }
    if let Some(probes) = diagnostic.get("probes").and_then(|v| v.as_object()) {
        lines.push("Probes:".to_string());
        if let Some(windows) = probes.get("windows.list") {
            if windows.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                let count = windows.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
                lines.push(format!("  windows.list: ok ({count})"));
            } else if let Some(err) = windows.get("error").and_then(|v| v.as_str()) {
                lines.push(format!("  windows.list: error ({err})"));
            } else {
                lines.push("  windows.list: unavailable".into());
            }
        }
        if let Some(ax) = probes.get("ax.trusted") {
            lines.push(format!("  ax.trusted: {ax}"));
        }
        lines.push(String::new());
    }
    if let Some(stores) = diagnostic.get("stores").and_then(|v| v.as_object()) {
        if !stores.is_empty() {
            lines.push("Stores:".to_string());
            for (key, val) in stores {
                lines.push(format!("  {key}: {val}"));
            }
            lines.push(String::new());
        }
    }
    if let Some(cmds) = diagnostic
        .get("config_commands")
        .and_then(|v| v.as_object())
    {
        if !cmds.is_empty() {
            lines.push("Config:".to_string());
            for (key, val) in cmds {
                if let Some(cmd) = val.as_str() {
                    lines.push(format!("  {key}: {cmd}"));
                }
            }
            lines.push(String::new());
        }
    }
    let notes = diagnostic
        .pointer("/settings/notes_root")
        .or_else(|| diagnostic.get("notes_root"))
        .and_then(|v| v.as_str());
    let projects = diagnostic
        .pointer("/settings/projects_roots")
        .or_else(|| diagnostic.get("projects_roots"))
        .and_then(|v| v.as_array());
    if notes.is_some() || projects.is_some() {
        lines.push("Roots:".to_string());
        lines.push(format!("  notes_root: {}", notes.unwrap_or("(not set)")));
        let proj = projects
            .map(|arr| {
                let joined: Vec<&str> = arr.iter().filter_map(|v| v.as_str()).collect();
                if joined.is_empty() {
                    "(none)".into()
                } else {
                    joined.join(", ")
                }
            })
            .unwrap_or_else(|| "(none)".into());
        lines.push(format!("  projects_roots: {proj}"));
        lines.push(String::new());
    }
    lines.join("\n")
}
