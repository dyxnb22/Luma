#!/usr/bin/env python3
"""Apply Phase 5 patches to monolithic engine.rs (from git HEAD)."""
from pathlib import Path

p = Path(__file__).resolve().parents[1] / "crates/luma-application/src/engine.rs"
text = p.read_text()
start = text.find("fn doctor_store_probes")
end = text.find("struct SearchTask")
if start == -1 or end == -1:
    raise SystemExit("doctor_store_probes block not found")
text = text[:start] + text[end:]
replacements = [
    (
        "    pub diagnostics: Option<Arc<dyn crate::ports::DiagnosticsSink>>,\n    /// Modules skipped",
        "    pub diagnostics: Option<Arc<dyn crate::ports::DiagnosticsSink>>,\n    pub storage_probe: Option<Arc<dyn crate::ports::StorageProbePort>>,\n    /// Modules skipped",
    ),
    (
        "    diagnostics: Option<Arc<dyn crate::ports::DiagnosticsSink>>,\n    skipped_modules: Vec<(String, String)>,\n    /// Serializes",
        "    diagnostics: Option<Arc<dyn crate::ports::DiagnosticsSink>>,\n    storage_probe: Option<Arc<dyn crate::ports::StorageProbePort>>,\n    skipped_modules: Vec<(String, String)>,\n    /// Serializes",
    ),
    (
        "                diagnostics: None,\n                skipped_modules: Vec::new(),",
        "                diagnostics: None,\n                storage_probe: None,\n                skipped_modules: Vec::new(),",
    ),
    (
        "            diagnostics: options.diagnostics,\n            skipped_modules: options.skipped_modules,",
        "            diagnostics: options.diagnostics,\n            storage_probe: options.storage_probe,\n            skipped_modules: options.skipped_modules,",
    ),
    (
        "                let store_probes = doctor_store_probes(settings_snapshot.is_some());",
        """                let settings_ok = settings_snapshot.is_some();
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
                        "clipboard": "missing",
                        "quicklinks": "missing",
                        "snippets": "missing",
                        "notes_index": "missing",
                    })
                };""",
    ),
    (
        "            Command::LoadPreview {",
        """            Command::GetSnapshot => {
                let (items, module_states) = {
                    let g = self.inner.lock().await;
                    let items: Vec<luma_protocol::SearchItemDto> =
                        g.results_by_id.values().map(luma_protocol::SearchItemDto::from).collect();
                    (items, g.module_states.clone())
                };
                let _ = self
                    .emit(Event::SnapshotLoaded {
                        items,
                        module_states,
                    })
                    .await;
            }
            Command::LoadPreview {""",
    ),
    (
        ") -> Result<(Vec<SearchItemDto>, Vec<Event>), String> {\n    let engine = Engine::with_settings(registry, settings);",
        ") -> Result<(Vec<SearchItemDto>, Vec<Event>), String> {\n    let triggers = registry.all_triggers();\n    let query = luma_domain::Query::normalize_for_cli(query, |token| {\n        is_meta_prefix(token) || triggers.iter().any(|t| t == token)\n    });\n    let engine = Engine::with_settings(registry, settings);",
    ),
    (
        "                diagnostics: Some(sink),\n                skipped_modules: Vec::new(),",
        "                diagnostics: Some(sink),\n                storage_probe: None,\n                skipped_modules: Vec::new(),",
    ),
]
for old, new in replacements:
    if old not in text:
        raise SystemExit(f"missing patch anchor: {old[:60]!r}...")
    text = text.replace(old, new, 1)
p.write_text(text)
print(f"patched {p}")
