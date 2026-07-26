use crate::cancel::await_unless_cancelled;
use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, LumaModule, ModuleManifest, ModuleState, PasteboardPort,
    SearchMode, SearchSink, ShortcutEntry, ShortcutsError, ShortcutsPort, WarmupContext,
    MAX_SHORTCUT_RESULTS,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{ActionDescriptorDto, Event, SearchItemDto};
use std::collections::HashMap;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

const MODULE_ID: &str = "luma.shortcuts";

pub struct ShortcutsModule {
    manifest: ModuleManifest,
    shortcuts: Arc<dyn ShortcutsPort>,
    pasteboard: Arc<dyn PasteboardPort>,
}

impl ShortcutsModule {
    pub fn with_deps(
        shortcuts: Arc<dyn ShortcutsPort>,
        pasteboard: Arc<dyn PasteboardPort>,
    ) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new(MODULE_ID),
                display_name: "Apple Shortcuts".into(),
                triggers: vec!["sc".into(), "shortcut".into(), "shortcuts".into()],
                default_enabled: true,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
                workbench: luma_application::WorkbenchMeta {
                    glyph: Some("S".into()),
                    suggested_query: Some("/sc ".into()),
                    empty_hint: Some("/sc · /sc folders · /sc folder <name>".into()),
                    supports_browse: false,
                },
            },
            shortcuts,
            pasteboard,
        }
    }
}

#[async_trait]
impl LumaModule for ShortcutsModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
        // Shortcuts enumeration can prompt or be slow; targeted visits own it.
        ModuleState::Ready
    }

    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        if cancel.is_cancelled() {
            return;
        }
        let rest = query.rest_raw();
        if rest.eq_ignore_ascii_case("folders") {
            let folders = match self
                .shortcuts
                .folders(query.limit.min(MAX_SHORTCUT_RESULTS), cancel.clone())
                .await
            {
                Ok(folders) => folders,
                Err(ShortcutsError::Cancelled) => return,
                Err(error) => {
                    send_shortcuts_error(&sink, error).await;
                    return;
                }
            };
            if folders.is_empty() {
                send_status(
                    &sink,
                    "sc:folders:empty",
                    "No custom Shortcuts folders",
                    "Create a folder in Apple Shortcuts if you want to filter by folder",
                    "status",
                )
                .await;
                return;
            }
            let mut occurrences = HashMap::<String, usize>::new();
            let upserts = folders
                .into_iter()
                .map(|folder| {
                    let occurrence = occurrences.entry(folder.clone()).or_default();
                    let item = folder_item(folder, *occurrence);
                    *occurrence += 1;
                    item
                })
                .collect();
            send_items(&sink, upserts).await;
            return;
        }

        let folder = strip_command(rest, "folder").filter(|value| !value.is_empty());
        if strip_command(rest, "folder").is_some() && folder.is_none() {
            send_status(
                &sink,
                "sc:folder:invalid",
                "Shortcuts folder is required",
                "Expected: /sc folder <exact folder name>",
                "command_error",
            )
            .await;
            return;
        }
        let filter = if folder.is_none() && !rest.trim().is_empty() {
            Some(rest.trim().to_lowercase())
        } else {
            None
        };
        let entries = match self
            .shortcuts
            .list(
                folder,
                query.limit.min(MAX_SHORTCUT_RESULTS),
                cancel.clone(),
            )
            .await
        {
            Ok(entries) => entries,
            Err(ShortcutsError::Cancelled) => return,
            Err(error) => {
                send_shortcuts_error(&sink, error).await;
                return;
            }
        };
        if cancel.is_cancelled() {
            return;
        }
        let entries = entries
            .into_iter()
            .filter(|entry| {
                filter
                    .as_ref()
                    .is_none_or(|needle| entry.name.to_lowercase().contains(needle))
            })
            .collect::<Vec<_>>();
        if entries.is_empty() {
            let (title, subtitle, kind) = if rest.trim().is_empty() {
                (
                    "No Apple Shortcuts found",
                    "Create a shortcut in the Shortcuts app, then retry /sc",
                    "not_configured",
                )
            } else {
                (
                    "No matching Apple Shortcuts",
                    "Try another name or folder",
                    "status",
                )
            };
            send_status(&sink, "sc:empty", title, subtitle, kind).await;
            return;
        }
        let mut counts = HashMap::<String, usize>::new();
        for entry in &entries {
            *counts.entry(entry.name.clone()).or_default() += 1;
        }
        let upserts = entries
            .into_iter()
            .map(|entry| {
                let duplicate = counts.get(&entry.name).copied().unwrap_or(0) > 1;
                shortcut_item(entry, duplicate)
            })
            .collect();
        send_items(&sink, upserts).await;
    }

    async fn actions(&self, result: &SearchItem) -> Vec<ActionDescriptor> {
        match result.kind.as_str() {
            "shortcut" => vec![
                safe_action("run", "Run"),
                safe_action("view", "View in Shortcuts"),
                safe_action("copy_name", "Copy name"),
            ],
            "shortcut_folder" => vec![safe_action("open_folder", "Open folder")],
            _ => vec![safe_action("noop", "OK")],
        }
    }

    async fn perform(&self, request: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        if request.action.id.as_str() == "noop" {
            return ActionOutcome::Success { message: None };
        }
        let Some(payload) = request.result.action_payload.as_ref() else {
            return invalid_action("missing Shortcuts action payload");
        };
        if request.action.id.as_str() == "open_folder" {
            let Some(folder) = payload.get("folder").and_then(|value| value.as_str()) else {
                return invalid_action("missing folder name");
            };
            return ActionOutcome::OpenSurface {
                query: format!("/sc folder {folder}"),
            };
        }
        let Some(name) = payload.get("name").and_then(|value| value.as_str()) else {
            return invalid_action("missing shortcut name");
        };
        match request.action.id.as_str() {
            "copy_name" => {
                match await_unless_cancelled(&cancel, self.pasteboard.write_text(name)).await {
                    None => ActionOutcome::Cancelled,
                    Some(Ok(())) => ActionOutcome::Success {
                        message: Some("copied shortcut name".into()),
                    },
                    Some(Err(error)) => ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: error.to_string(),
                            retryable: true,
                        },
                    },
                }
            }
            "view" => match self.shortcuts.view(name, cancel.clone()).await {
                Ok(()) => ActionOutcome::Success {
                    message: Some(format!("opened {name} in Shortcuts")),
                },
                Err(error) => shortcuts_outcome(error),
            },
            "run" => {
                let plan = match self.shortcuts.run_plan(name, cancel.clone()).await {
                    Ok(plan) => plan,
                    Err(error) => return shortcuts_outcome(error),
                };
                if cancel.is_cancelled() {
                    return ActionOutcome::Cancelled;
                }
                if plan.shortcut.name != name {
                    return ActionOutcome::Failed {
                        kind: FailureKind::Conflict {
                            reason: "shortcut identity changed; search again".into(),
                        },
                    };
                }
                ActionOutcome::InteractiveTerminal {
                    program: plan.program,
                    args: plan.args,
                    record_alias: None,
                }
            }
            _ => invalid_action("unknown Shortcuts action"),
        }
    }

    async fn teardown(&self) {}
}

fn strip_command<'a>(input: &'a str, command: &str) -> Option<&'a str> {
    let (word, rest) = input.split_once(char::is_whitespace).unwrap_or((input, ""));
    word.eq_ignore_ascii_case(command).then_some(rest.trim())
}

fn shortcut_item(entry: ShortcutEntry, duplicate: bool) -> SearchItemDto {
    SearchItemDto {
        id: format!("sc:{}", entry.id),
        module_id: MODULE_ID.into(),
        title: entry.name.clone(),
        subtitle: Some(if duplicate {
            "Duplicate exact name — Run/View will refuse until names are unique".into()
        } else {
            "Apple Shortcut".into()
        }),
        kind: "shortcut".into(),
        score: 75.0,
        primary_action_id: "run".into(),
        primary_action_label: "Run".into(),
        secondary_actions: vec![
            action_dto("view", "View in Shortcuts"),
            action_dto("copy_name", "Copy name"),
        ],
        action_payload: Some(serde_json::json!({ "name": entry.name })),
        ..Default::default()
    }
}

fn folder_item(folder: String, occurrence: usize) -> SearchItemDto {
    SearchItemDto {
        id: format!("sc:folder:{:016x}", stable_folder_hash(&folder, occurrence)),
        module_id: MODULE_ID.into(),
        title: folder.clone(),
        subtitle: Some("Custom Shortcuts folder".into()),
        kind: "shortcut_folder".into(),
        score: 70.0,
        primary_action_id: "open_folder".into(),
        primary_action_label: "Open folder".into(),
        action_payload: Some(serde_json::json!({ "folder": folder })),
        ..Default::default()
    }
}

fn stable_folder_hash(folder: &str, occurrence: usize) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in folder
        .as_bytes()
        .iter()
        .copied()
        .chain([0])
        .chain(occurrence.to_le_bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn action_dto(id: &str, label: &str) -> ActionDescriptorDto {
    ActionDescriptorDto {
        id: id.into(),
        label: label.into(),
        risk: ActionRisk::Safe,
        confirmation: false,
    }
}

fn safe_action(id: &str, label: &str) -> ActionDescriptor {
    ActionDescriptor {
        id: ActionId::new(id),
        label: label.into(),
        risk: ActionRisk::Safe,
        confirmation: false,
    }
}

fn invalid_action(message: &str) -> ActionOutcome {
    ActionOutcome::Failed {
        kind: FailureKind::InvalidInput {
            field: "action".into(),
            message: message.into(),
        },
    }
}

fn shortcuts_outcome(error: ShortcutsError) -> ActionOutcome {
    if error == ShortcutsError::Cancelled {
        return ActionOutcome::Cancelled;
    }
    ActionOutcome::Failed {
        kind: match error {
            ShortcutsError::Unavailable => FailureKind::Unavailable {
                reason: "/usr/bin/shortcuts is unavailable".into(),
                retryable: false,
            },
            ShortcutsError::CommandFailed(reason) => FailureKind::Unavailable {
                reason,
                retryable: true,
            },
            ShortcutsError::Timeout => FailureKind::Timeout {
                operation: "Shortcuts command".into(),
            },
            ShortcutsError::OutputTooLarge(limit) => FailureKind::Unavailable {
                reason: format!("Shortcuts output exceeded {limit} bytes"),
                retryable: false,
            },
            ShortcutsError::NotFound => FailureKind::NotFound {
                entity: "Apple Shortcut".into(),
            },
            ShortcutsError::Ambiguous => FailureKind::Conflict {
                reason: "multiple shortcuts have the same exact name; rename one first".into(),
            },
            ShortcutsError::Cancelled => unreachable!(),
        },
    }
}

async fn send_items(sink: &SearchSink, upserts: Vec<SearchItemDto>) {
    let _ = sink
        .send(Event::ResultsChunk {
            request_id: String::new(),
            sequence: 1,
            upserts,
            removed_ids: vec![],
        })
        .await;
}

async fn send_status(sink: &SearchSink, id: &str, title: &str, subtitle: &str, kind: &str) {
    send_items(
        sink,
        vec![SearchItemDto {
            id: id.into(),
            module_id: MODULE_ID.into(),
            title: title.into(),
            subtitle: Some(subtitle.into()),
            kind: kind.into(),
            score: 0.0,
            primary_action_id: "noop".into(),
            primary_action_label: "OK".into(),
            ..Default::default()
        }],
    )
    .await;
}

async fn send_shortcuts_error(sink: &SearchSink, error: ShortcutsError) {
    match error {
        ShortcutsError::Unavailable => {
            send_status(
                sink,
                "sc:unavailable",
                "Apple Shortcuts is unavailable",
                "/usr/bin/shortcuts could not be used",
                "unavailable",
            )
            .await
        }
        ShortcutsError::Cancelled => {}
        ShortcutsError::NotFound => {
            send_status(
                sink,
                "sc:not-found",
                "Shortcut or folder no longer exists",
                "Search again before acting",
                "command_error",
            )
            .await
        }
        other => {
            send_status(
                sink,
                "sc:failed",
                "Shortcuts command failed",
                &other.to_string(),
                "command_error",
            )
            .await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_application::{FakePasteboard, FakeShortcuts, ShortcutEntry};

    fn entry(id: &str, name: &str) -> ShortcutEntry {
        ShortcutEntry {
            id: id.into(),
            name: name.into(),
        }
    }

    #[tokio::test]
    async fn warmup_does_not_enumerate_or_run_shortcuts() {
        let shortcuts = Arc::new(FakeShortcuts::new(vec![entry("one", "Morning")], vec![]));
        let module = ShortcutsModule::with_deps(shortcuts.clone(), Arc::new(FakePasteboard::new()));
        assert_eq!(
            module
                .warmup(WarmupContext {
                    cancel: CancellationToken::new()
                })
                .await,
            ModuleState::Ready
        );
        assert!(shortcuts.run_calls.lock().unwrap().is_empty());
        assert!(shortcuts.view_calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn run_returns_exact_interactive_terminal_plan() {
        let shortcuts = Arc::new(FakeShortcuts::new(
            vec![entry("one", "Morning Routine")],
            vec![],
        ));
        let module = ShortcutsModule::with_deps(shortcuts, Arc::new(FakePasteboard::new()));
        let outcome = module
            .perform(
                ActionRequest {
                    result: shortcut_item(entry("one", "Morning Routine"), false).into_domain(),
                    action: safe_action("run", "Run"),
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert_eq!(
            outcome,
            ActionOutcome::InteractiveTerminal {
                program: "/usr/bin/shortcuts".into(),
                args: vec!["run".into(), "Morning Routine".into()],
                record_alias: None,
            }
        );
    }

    #[tokio::test]
    async fn stale_and_duplicate_names_never_run() {
        for entries in [
            vec![entry("new", "Renamed")],
            vec![entry("one", "Morning"), entry("two", "Morning")],
        ] {
            let shortcuts = Arc::new(FakeShortcuts::new(entries, vec![]));
            let module =
                ShortcutsModule::with_deps(shortcuts.clone(), Arc::new(FakePasteboard::new()));
            let outcome = module
                .perform(
                    ActionRequest {
                        result: shortcut_item(entry("old", "Morning"), false).into_domain(),
                        action: safe_action("run", "Run"),
                        confirmation: false,
                    },
                    CancellationToken::new(),
                )
                .await;
            assert!(matches!(outcome, ActionOutcome::Failed { .. }));
            assert!(shortcuts.run_calls.lock().unwrap().is_empty());
        }
    }

    #[tokio::test]
    async fn cancellation_prevents_view_and_run() {
        let shortcuts = Arc::new(FakeShortcuts::new(vec![entry("one", "Morning")], vec![]));
        let module = ShortcutsModule::with_deps(shortcuts.clone(), Arc::new(FakePasteboard::new()));
        let cancel = CancellationToken::new();
        cancel.cancel();
        let outcome = module
            .perform(
                ActionRequest {
                    result: shortcut_item(entry("one", "Morning"), false).into_domain(),
                    action: safe_action("view", "View"),
                    confirmation: false,
                },
                cancel,
            )
            .await;
        assert_eq!(outcome, ActionOutcome::Cancelled);
        assert!(shortcuts.view_calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn empty_and_unavailable_are_distinct() {
        let empty = Arc::new(FakeShortcuts::new(vec![], vec![]));
        let module = ShortcutsModule::with_deps(empty, Arc::new(FakePasteboard::new()));
        let (sink, mut receiver) = tokio::sync::mpsc::channel(2);
        module
            .search(
                Query::parse_with_prefixes_strict("/sc ", 50, |value| value == "sc"),
                sink,
                CancellationToken::new(),
            )
            .await;
        let Event::ResultsChunk { upserts, .. } = receiver.recv().await.unwrap() else {
            panic!("expected result");
        };
        assert_eq!(upserts[0].kind, "not_configured");

        let unavailable = Arc::new(FakeShortcuts::new(vec![], vec![]));
        unavailable.fail_with(ShortcutsError::Unavailable);
        let module = ShortcutsModule::with_deps(unavailable, Arc::new(FakePasteboard::new()));
        let (sink, mut receiver) = tokio::sync::mpsc::channel(2);
        module
            .search(
                Query::parse_with_prefixes_strict("/sc ", 50, |value| value == "sc"),
                sink,
                CancellationToken::new(),
            )
            .await;
        let Event::ResultsChunk { upserts, .. } = receiver.recv().await.unwrap() else {
            panic!("expected result");
        };
        assert_eq!(upserts[0].kind, "unavailable");
    }
}
