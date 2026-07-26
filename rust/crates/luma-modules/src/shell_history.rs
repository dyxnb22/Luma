use crate::cancel::await_unless_cancelled;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use luma_application::{
    ActionOutcome, ActionRequest, LumaModule, ModuleManifest, ModuleState, PasteboardPort,
    SearchMode, SearchSink, ShellHistoryEntry, ShellHistoryError, ShellHistoryPort, WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{Event, SearchItemDto};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

const MODULE_ID: &str = "luma.shell_history";

pub struct ShellHistoryModule {
    manifest: ModuleManifest,
    history: Arc<dyn ShellHistoryPort>,
    pasteboard: Arc<dyn PasteboardPort>,
}

impl ShellHistoryModule {
    pub fn with_deps(
        history: Arc<dyn ShellHistoryPort>,
        pasteboard: Arc<dyn PasteboardPort>,
    ) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new(MODULE_ID),
                display_name: "Shell Recall".into(),
                triggers: vec!["hist".into(), "history".into()],
                default_enabled: true,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
                workbench: luma_application::WorkbenchMeta {
                    glyph: Some("H".into()),
                    suggested_query: Some("/hist ".into()),
                    empty_hint: Some("/hist recent · /hist <query>".into()),
                    supports_browse: false,
                },
            },
            history,
            pasteboard,
        }
    }
}

#[async_trait]
impl LumaModule for ShellHistoryModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
        ModuleState::Ready
    }

    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        if cancel.is_cancelled() {
            return;
        }
        let snapshot = match self.history.read(cancel.clone()).await {
            Ok(snapshot) => snapshot,
            Err(ShellHistoryError::Cancelled) => return,
            Err(ShellHistoryError::NotConfigured) => {
                send_status(
                    &sink,
                    "hist:not-configured",
                    "zsh history is not configured",
                    "~/.zsh_history does not exist",
                    "not_configured",
                )
                .await;
                return;
            }
            Err(ShellHistoryError::Unavailable(reason)) => {
                send_status(
                    &sink,
                    "hist:unavailable",
                    "zsh history is unavailable",
                    &reason,
                    "unavailable",
                )
                .await;
                return;
            }
        };
        if cancel.is_cancelled() {
            return;
        }
        let rest = query.rest_raw().trim();
        let needle = if rest.eq_ignore_ascii_case("recent") {
            ""
        } else {
            rest
        }
        .to_lowercase();
        let mut matches = snapshot
            .entries
            .into_iter()
            .filter(|entry| needle.is_empty() || entry.command.to_lowercase().contains(&needle))
            .take(query.limit)
            .collect::<Vec<_>>();
        if matches.is_empty() {
            let subtitle = if snapshot.hidden_count > 0 {
                format!(
                    "{} private or oversized entr{} hidden",
                    snapshot.hidden_count,
                    if snapshot.hidden_count == 1 {
                        "y was"
                    } else {
                        "ies were"
                    }
                )
            } else if needle.is_empty() {
                "The bounded zsh history tail contains no commands".into()
            } else {
                "Try another search term".into()
            };
            send_status(
                &sink,
                "hist:empty",
                "No privacy-safe shell history matches",
                &subtitle,
                "status",
            )
            .await;
            return;
        }
        let hidden_count = snapshot.hidden_count;
        let upserts = matches
            .drain(..)
            .enumerate()
            .map(|(index, entry)| history_item(entry, (index == 0).then_some(hidden_count)))
            .collect();
        let _ = sink
            .send(Event::ResultsChunk {
                request_id: String::new(),
                sequence: 1,
                upserts,
                removed_ids: vec![],
            })
            .await;
    }

    async fn actions(&self, result: &SearchItem) -> Vec<ActionDescriptor> {
        if result.kind == "shell_history_command" {
            vec![safe_action("copy", "Copy command")]
        } else {
            vec![safe_action("noop", "OK")]
        }
    }

    async fn perform(&self, request: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        if request.action.id.as_str() == "noop" {
            return ActionOutcome::Success { message: None };
        }
        if request.action.id.as_str() != "copy" {
            return invalid_action("Shell Recall is copy-only");
        }
        let Some(id) = request
            .result
            .action_payload
            .as_ref()
            .and_then(|payload| payload.get("id"))
            .and_then(|value| value.as_str())
        else {
            return invalid_action("missing shell history identity");
        };
        let snapshot = match self.history.read(cancel.clone()).await {
            Ok(snapshot) => snapshot,
            Err(error) => return history_outcome(error),
        };
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        let Some(entry) = snapshot.entries.into_iter().find(|entry| entry.id == id) else {
            return ActionOutcome::Failed {
                kind: FailureKind::Conflict {
                    reason: "shell history changed; search again before copying".into(),
                },
            };
        };
        match await_unless_cancelled(&cancel, self.pasteboard.write_text(&entry.command)).await {
            None => ActionOutcome::Cancelled,
            Some(Ok(())) => ActionOutcome::Success {
                message: Some("copied shell history command".into()),
            },
            Some(Err(error)) => ActionOutcome::Failed {
                kind: FailureKind::Unavailable {
                    reason: error.to_string(),
                    retryable: true,
                },
            },
        }
    }

    async fn teardown(&self) {}
}

fn history_item(entry: ShellHistoryEntry, hidden_count: Option<usize>) -> SearchItemDto {
    let mut subtitle = entry
        .timestamp
        .and_then(|timestamp| DateTime::<Utc>::from_timestamp(timestamp, 0))
        .map(|timestamp| timestamp.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| "zsh history".into());
    if hidden_count.is_some_and(|count| count > 0) {
        subtitle.push_str(&format!(
            " · {} private/oversized hidden",
            hidden_count.unwrap_or_default()
        ));
    }
    SearchItemDto {
        id: format!("hist:{}", entry.id),
        module_id: MODULE_ID.into(),
        title: entry.command,
        subtitle: Some(subtitle),
        kind: "shell_history_command".into(),
        score: 75.0,
        primary_action_id: "copy".into(),
        primary_action_label: "Copy command".into(),
        action_payload: Some(serde_json::json!({ "id": entry.id })),
        ..Default::default()
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

fn history_outcome(error: ShellHistoryError) -> ActionOutcome {
    match error {
        ShellHistoryError::Cancelled => ActionOutcome::Cancelled,
        ShellHistoryError::NotConfigured => ActionOutcome::Failed {
            kind: FailureKind::NotConfigured {
                remediation: "~/.zsh_history does not exist".into(),
            },
        },
        ShellHistoryError::Unavailable(reason) => ActionOutcome::Failed {
            kind: FailureKind::Unavailable {
                reason,
                retryable: true,
            },
        },
    }
}

async fn send_status(sink: &SearchSink, id: &str, title: &str, subtitle: &str, kind: &str) {
    let _ = sink
        .send(Event::ResultsChunk {
            request_id: String::new(),
            sequence: 1,
            upserts: vec![SearchItemDto {
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
            removed_ids: vec![],
        })
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_application::{FakePasteboard, FakeShellHistory, ShellHistorySnapshot};

    fn entry(id: &str, command: &str) -> ShellHistoryEntry {
        ShellHistoryEntry {
            id: id.into(),
            command: command.into(),
            timestamp: Some(1_700_000_000),
            duration_seconds: Some(0),
        }
    }

    #[tokio::test]
    async fn search_is_read_only_and_copy_revalidates_identity() {
        let history = Arc::new(FakeShellHistory::new(ShellHistorySnapshot {
            entries: vec![entry("one", "git status")],
            hidden_count: 2,
        }));
        let pasteboard = Arc::new(FakePasteboard::new());
        let module = ShellHistoryModule::with_deps(history.clone(), pasteboard.clone());
        let item = history_item(entry("one", "git status"), Some(2)).into_domain();
        let outcome = module
            .perform(
                ActionRequest {
                    result: item,
                    action: safe_action("copy", "Copy"),
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(outcome, ActionOutcome::Success { .. }));
        assert_eq!(pasteboard.last_text().as_deref(), Some("git status"));

        let stale = history_item(entry("missing", "rm -rf nope"), None).into_domain();
        let outcome = module
            .perform(
                ActionRequest {
                    result: stale,
                    action: safe_action("copy", "Copy"),
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(
            outcome,
            ActionOutcome::Failed {
                kind: FailureKind::Conflict { .. }
            }
        ));
    }

    #[tokio::test]
    async fn cancellation_prevents_clipboard_write() {
        let history = Arc::new(FakeShellHistory::new(ShellHistorySnapshot {
            entries: vec![entry("one", "git status")],
            hidden_count: 0,
        }));
        let pasteboard = Arc::new(FakePasteboard::new());
        let module = ShellHistoryModule::with_deps(history, pasteboard.clone());
        let cancel = CancellationToken::new();
        cancel.cancel();
        let outcome = module
            .perform(
                ActionRequest {
                    result: history_item(entry("one", "git status"), None).into_domain(),
                    action: safe_action("copy", "Copy"),
                    confirmation: false,
                },
                cancel,
            )
            .await;
        assert_eq!(outcome, ActionOutcome::Cancelled);
        assert_eq!(pasteboard.last_text(), None);
    }

    #[tokio::test]
    async fn not_configured_and_unavailable_are_distinct() {
        for (error, expected) in [
            (ShellHistoryError::NotConfigured, "not_configured"),
            (
                ShellHistoryError::Unavailable("fixture".into()),
                "unavailable",
            ),
        ] {
            let history = Arc::new(FakeShellHistory::new(ShellHistorySnapshot {
                entries: vec![],
                hidden_count: 0,
            }));
            history.fail_with(error);
            let module = ShellHistoryModule::with_deps(history, Arc::new(FakePasteboard::new()));
            let (sink, mut receiver) = tokio::sync::mpsc::channel(2);
            module
                .search(
                    Query::parse_with_prefixes_strict("/hist ", 50, |value| value == "hist"),
                    sink,
                    CancellationToken::new(),
                )
                .await;
            let Event::ResultsChunk { upserts, .. } = receiver.recv().await.unwrap() else {
                panic!("expected result");
            };
            assert_eq!(upserts[0].kind, expected);
        }
    }

    #[tokio::test]
    async fn no_execution_action_is_exposed() {
        let module = ShellHistoryModule::with_deps(
            Arc::new(FakeShellHistory::new(ShellHistorySnapshot {
                entries: vec![],
                hidden_count: 0,
            })),
            Arc::new(FakePasteboard::new()),
        );
        let actions = module
            .actions(&history_item(entry("one", "echo hello"), None).into_domain())
            .await;
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].id.as_str(), "copy");
    }
}
