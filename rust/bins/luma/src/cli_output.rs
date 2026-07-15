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
