use crate::ports::{RecallObject, MAX_RECALL_TITLE_CHARS};
use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ModuleId, SearchItem};
use std::collections::{BTreeMap, HashMap};

pub(crate) const GLOBAL_RESULTS_PER_MODULE: usize = 12;
pub(crate) const MAX_GLOBAL_RESULTS: usize = 60;
pub(crate) const HUB_CONTINUE_LIMIT: usize = 5;

pub(crate) fn recall_object_from_item(item: &SearchItem, now_unix: i64) -> Option<RecallObject> {
    if item.module_id.as_str() == "luma.system"
        || matches!(
            item.kind.as_str(),
            "status"
                | "warning"
                | "warming"
                | "unavailable"
                | "permission_required"
                | "not_configured"
                | "command_error"
                | "onboarding"
        )
        || item.primary_action.id.as_str() == "noop"
    {
        return None;
    }
    let title = privacy_safe_title(item);
    Some(RecallObject {
        object_id: item.id.as_str().to_string(),
        module_id: item.module_id.as_str().to_string(),
        kind: item.kind.clone(),
        primary_action: item.primary_action.id.as_str().to_string(),
        title,
        project_path: project_association(item),
        use_count: 0,
        last_used_at: now_unix,
    })
}

fn privacy_safe_title(item: &SearchItem) -> String {
    // These source modules may contain private values in their titles. Recall needs only an
    // actionable identity, not a second copy of the source text/configuration.
    let generic = match item.module_id.as_str() {
        "luma.clipboard" => Some("Clipboard item"),
        "luma.snippets" => Some("Snippet"),
        "luma.ssh" => Some("SSH connection"),
        _ => None,
    };
    let raw = generic.unwrap_or(item.title.as_str());
    let compact = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut title: String = compact.chars().take(MAX_RECALL_TITLE_CHARS).collect();
    if title.is_empty() {
        title = kind_label(&item.kind).into();
    }
    title
}

fn project_association(item: &SearchItem) -> Option<String> {
    if let Some(path) = item
        .action_payload
        .as_ref()
        .and_then(|payload| payload.get("project_path"))
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
    {
        return Some(path.to_string());
    }
    (item.module_id.as_str() == "luma.projects")
        .then(|| item.id.as_str().strip_prefix("proj:").map(str::to_string))
        .flatten()
}

pub(crate) fn apply_recall_score(
    item: &mut SearchItem,
    records: &HashMap<String, RecallObject>,
    now_unix: i64,
    recent_project: Option<&str>,
) {
    let Some(record) = records.get(item.id.as_str()) else {
        return;
    };
    let age = now_unix.saturating_sub(record.last_used_at).max(0);
    let recency = match age {
        0..=3_600 => 28.0,
        3_601..=86_400 => 20.0,
        86_401..=604_800 => 12.0,
        604_801..=2_592_000 => 6.0,
        _ => 1.0,
    };
    let frequency = ((record.use_count.max(1) as f64).log2() + 1.0).min(6.0) * 4.0;
    let context = match (recent_project, record.project_path.as_deref()) {
        (Some(current), Some(associated)) if current == associated => 10.0,
        _ => 0.0,
    };
    // Existing module scores are their text-relevance signal. The additive terms remain small
    // enough that a good text match wins over an unrelated old item.
    item.score += recency + frequency + context;
}

/// Round-robin selection makes global search type-fair even when one catalogue has hundreds of
/// textual matches. The stable module/id ordering means identical inputs always rank identically.
pub(crate) fn fair_global_results(mut items: Vec<SearchItem>) -> Vec<SearchItem> {
    let mut by_module: BTreeMap<String, Vec<SearchItem>> = BTreeMap::new();
    for item in items.drain(..) {
        by_module
            .entry(item.module_id.as_str().to_string())
            .or_default()
            .push(item);
    }
    for module_items in by_module.values_mut() {
        module_items.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.as_str().cmp(b.id.as_str()))
        });
        module_items.truncate(GLOBAL_RESULTS_PER_MODULE);
    }
    let mut result = Vec::new();
    for index in 0..GLOBAL_RESULTS_PER_MODULE {
        for module_items in by_module.values() {
            if let Some(item) = module_items.get(index) {
                result.push(item.clone());
                if result.len() == MAX_GLOBAL_RESULTS {
                    break;
                }
            }
        }
        if result.len() == MAX_GLOBAL_RESULTS {
            break;
        }
    }
    for (index, item) in result.iter_mut().enumerate() {
        // TUI sorting is score-based. Preserve the intentionally fair total order there too.
        item.score = (MAX_GLOBAL_RESULTS.saturating_sub(index)) as f64 * 10_000.0;
    }
    result
}

pub(crate) fn hub_item(record: &RecallObject) -> SearchItem {
    SearchItem {
        id: luma_domain::ResultId::new(record.object_id.clone()),
        module_id: ModuleId::new(record.module_id.clone()),
        title: record.title.clone(),
        subtitle: Some(format!(
            "{} · used {}",
            kind_label(&record.kind),
            record.use_count
        )),
        kind: record.kind.clone(),
        score: 0.0,
        primary_action: ActionDescriptor {
            id: ActionId::new(record.primary_action.clone()),
            label: "Continue".into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        },
        secondary_actions: vec![],
        ui_intent: None,
        action_payload: record
            .project_path
            .as_ref()
            .map(|project_path| serde_json::json!({ "project_path": project_path })),
    }
}

pub(crate) fn kind_label(kind: &str) -> &str {
    match kind {
        "window" => "Window",
        "app" => "App",
        "project" => "Project",
        "note" => "Note",
        "recipe" => "Command",
        "ssh_host" => "SSH",
        "clipboard" => "Clipboard",
        "snippet" => "Snippet",
        "git_repo" => "Git repository",
        "runtime_listener" => "Runtime",
        other if !other.is_empty() => other,
        _ => "Item",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_domain::{ResultId, SearchItem};

    fn item(module: &str, id: &str, score: f64) -> SearchItem {
        SearchItem {
            id: ResultId::new(id),
            module_id: ModuleId::new(module),
            title: id.into(),
            subtitle: None,
            kind: "project".into(),
            score,
            primary_action: ActionDescriptor {
                id: ActionId::new("open"),
                label: "Open".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: None,
        }
    }

    #[test]
    fn ranking_prefers_recent_frequent_item_without_erasing_text_score() {
        let mut record = HashMap::new();
        record.insert(
            "a".into(),
            RecallObject {
                object_id: "a".into(),
                module_id: "luma.projects".into(),
                kind: "project".into(),
                primary_action: "open".into(),
                title: "A".into(),
                project_path: Some("/p".into()),
                use_count: 8,
                last_used_at: 100,
            },
        );
        let mut candidate = item("luma.projects", "a", 70.0);
        apply_recall_score(&mut candidate, &record, 200, Some("/p"));
        assert!(candidate.score > 100.0);
    }

    #[test]
    fn fair_selection_does_not_allow_one_module_to_flood_top_rows() {
        let mut items = (0..20)
            .map(|i| item("luma.notes", &format!("n{i}"), 100.0 - i as f64))
            .collect::<Vec<_>>();
        items.push(item("luma.projects", "p", 1.0));
        items.push(item("luma.ssh", "s", 1.0));
        let ranked = fair_global_results(items);
        assert_eq!(ranked[0].module_id.as_str(), "luma.notes");
        assert_eq!(ranked[1].module_id.as_str(), "luma.projects");
        assert_eq!(ranked[2].module_id.as_str(), "luma.ssh");
    }

    #[test]
    fn clipboard_recall_uses_generic_label() {
        let mut clipboard = item("luma.clipboard", "clip:1", 1.0);
        clipboard.title = "very private body".into();
        assert_eq!(
            recall_object_from_item(&clipboard, 1).unwrap().title,
            "Clipboard item"
        );
    }
}
