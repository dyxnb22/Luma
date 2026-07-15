use super::*;
use luma_protocol::Event;

fn doctor_launch_context() -> serde_json::Value {
    let executable = std::env::current_exe()
        .ok()
        .map(|p| p.display().to_string());
    #[cfg(unix)]
    {
        let parent_pid = std::os::unix::process::parent_id();
        let parent_name = unix_process_comm(parent_pid);
        let host = parent_name
            .clone()
            .unwrap_or_else(|| "the app that launched Luma".into());
        serde_json::json!({
            "executable": executable,
            "parent_pid": parent_pid,
            "parent_name": parent_name,
            "guidance": format!(
                "Grant Accessibility to {host} in System Settings → Privacy & Security → Accessibility, then re-run doctor."
            ),
        })
    }
    #[cfg(not(unix))]
    {
        serde_json::json!({ "executable": executable })
    }
}

fn doctor_paths() -> serde_json::Value {
    serde_json::json!({
        "support_dir": luma_storage::luma_next_support_dir().ok().map(|p| p.display().to_string()),
        "logs_dir": luma_storage::luma_next_logs_dir().ok().map(|p| p.display().to_string()),
        "diagnostics_dir": luma_storage::luma_next_diagnostics_dir().ok().map(|p| p.display().to_string()),
    })
}

impl Engine {
    pub(super) async fn handle_run_doctor(&self) {
        let (rows, settings_snapshot, module_states) = {
            let g = self.inner.lock().await;
            let rows = g.registry.list();
            let settings_snapshot = self
                .settings
                .as_ref()
                .and_then(|s| s.load_or_default().ok());
            let module_states = g.module_states.clone();
            (rows, settings_snapshot, module_states)
        };
        let modules = rows
            .iter()
            .map(|(id, enabled, name)| {
                let state = module_states.get(id).cloned().unwrap_or_else(|| {
                    if *enabled {
                        "enabled".into()
                    } else {
                        "disabled".into()
                    }
                });
                serde_json::json!({
                    "id": id,
                    "enabled": enabled,
                    "name": name,
                    "state": state,
                })
            })
            .collect::<Vec<_>>();
        let notes_root_configured = settings_snapshot
            .as_ref()
            .and_then(|s| s.notes_root.as_ref())
            .is_some();
        let projects_roots = settings_snapshot
            .as_ref()
            .map(|s| s.projects_roots.len())
            .unwrap_or(0);
        let mut remediation = Vec::new();
        if !notes_root_configured {
            remediation.push("Notes: luma config set --notes-root ~/Notes".to_string());
        }
        if projects_roots == 0 {
            remediation.push("Projects: luma config set --projects-root ~/dev".to_string());
        }
        for (id, reason) in &self.skipped_modules {
            remediation.push(format!(
                "Repair store for {id} ({reason}), then restart Luma"
            ));
        }
        remediation.push("Grant Accessibility if paste/snippets paste / window focus fails".into());
        remediation
            .push("Windows titles may need Screen Recording; focus needs Accessibility".into());
        remediation.push("Notes excludes: luma config set --notes-exclude 'private/*'".into());
        let skipped = self
            .skipped_modules
            .iter()
            .map(|(id, reason)| serde_json::json!({ "id": id, "reason": reason }))
            .collect::<Vec<_>>();
        let settings_ok = settings_snapshot.is_some();
        let store_probes = if let Some(probe) = &self.storage_probe {
            tokio::task::spawn_blocking({
                let probe = probe.clone();
                move || probe.probe_stores(settings_ok)
            })
            .await
            .unwrap_or_else(|_| serde_json::json!({"error": "store probe task failed"}))
        } else {
            serde_json::json!({
                "settings": if settings_ok { "ok" } else { "missing" },
                "clipboard": "unconfigured",
                "quicklinks": "unconfigured",
                "snippets": "unconfigured",
                "notes_index": "unconfigured",
            })
        };
        let diagnostic = serde_json::json!({
            "doctor": true,
            "modules": modules,
            "skipped_modules": skipped,
            "paths": doctor_paths(),
            "launch": doctor_launch_context(),
            "settings": {
                "configured": settings_ok,
                "settings_version": settings_snapshot.as_ref().map(|s| s.settings_version),
                "notes_root_configured": notes_root_configured,
                "notes_root": settings_snapshot.as_ref().and_then(|s| s.notes_root.clone()),
                "projects_roots": settings_snapshot.as_ref().map(|s| s.projects_roots.clone()).unwrap_or_default(),
                "notes_exclude_patterns": settings_snapshot.as_ref().map(|s| s.notes_exclude_patterns.clone()).unwrap_or_default(),
                "clipboard_retention_days": settings_snapshot.as_ref().map(|s| s.clipboard_retention_days),
            },
            "config_commands": {
                "notes_root": "luma config set --notes-root ~/Notes",
                "projects_roots": "luma config set --projects-root ~/dev",
                "notes_exclude": "luma config set --notes-exclude 'private/*'",
            },
            "stores": {
                "settings": store_probes["settings"].clone(),
                "diagnostics": if self.diagnostics.is_some() { "ok" } else { "missing" },
                "clipboard": store_probes["clipboard"].clone(),
                "quicklinks": store_probes["quicklinks"].clone(),
                "snippets": store_probes["snippets"].clone(),
                "notes_index": store_probes["notes_index"].clone(),
            },
            "remediation": remediation,
        });
        let _ = self.emit(Event::DiagnosticRaised { diagnostic }).await;
    }
}
