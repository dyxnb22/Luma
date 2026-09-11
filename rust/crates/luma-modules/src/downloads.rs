use crate::cancel::await_unless_cancelled;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use luma_application::{
    ActionOutcome, ActionRequest, DownloadCategory, DownloadEntry, DownloadsError, DownloadsFilter,
    DownloadsPort, LumaModule, ModuleManifest, ModuleState, OpenPathPort, PasteboardPort,
    SearchMode, SearchSink, WarmupContext, MAX_DOWNLOAD_ENTRIES,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{ActionDescriptorDto, Event, SearchItemDto};
use std::path::Path;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

const MODULE_ID: &str = "luma.downloads";

pub struct DownloadsModule {
    manifest: ModuleManifest,
    downloads: Arc<dyn DownloadsPort>,
    opener: Arc<dyn OpenPathPort>,
    pasteboard: Arc<dyn PasteboardPort>,
}

impl DownloadsModule {
    /// Canonical command discovery owned by this module, including unavailable fallbacks.
    pub fn command_specs() -> Vec<luma_application::CommandSpec> {
        vec![
            crate::ux::command_spec(
                "/dl [recent|large|query]",
                "List recent downloads or filter direct children",
                "/dl ",
                Some("/dl recent"),
            ),
            crate::ux::command_spec(
                "/dl old <days>d",
                "List direct children older than a bounded duration",
                "/dl old ",
                Some("/dl old 30d"),
            ),
            crate::ux::command_spec(
                "/dl type <archive|image|video|document|installer>",
                "Filter Downloads by file category",
                "/dl type ",
                Some("/dl type image"),
            ),
            crate::ux::command_spec(
                "/dl rename <result-id> | <new-name>",
                "Prepare an explicit rename; extension changes confirm",
                "/dl rename ",
                Some("/dl rename dl:abc | report.pdf"),
            ),
        ]
    }

    pub fn with_deps(
        downloads: Arc<dyn DownloadsPort>,
        opener: Arc<dyn OpenPathPort>,
        pasteboard: Arc<dyn PasteboardPort>,
    ) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new(MODULE_ID),
                display_name: "Downloads Inbox".into(),
                triggers: vec!["dl".into(), "downloads".into()],
                default_enabled: true,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
                workbench: luma_application::WorkbenchMeta {
                    glyph: Some("D".into()),
                    suggested_query: Some("/dl ".into()),
                    empty_hint: Some("/dl recent · large · old 30d · type image".into()),
                    supports_browse: false,
                    commands: Self::command_specs(),
                },
            },
            downloads,
            opener,
            pasteboard,
        }
    }
}

#[async_trait]
impl LumaModule for DownloadsModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
        // Downloads is deliberately on-demand: no watcher and no warmup scan.
        ModuleState::Ready
    }

    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        if cancel.is_cancelled() {
            return;
        }
        let rest = query.rest_raw();
        if let Some(rename) = strip_command(rest, "rename") {
            self.search_rename(rename, &sink, cancel).await;
            return;
        }
        let filter = match parse_filter(rest) {
            Ok(filter) => filter,
            Err(message) => {
                send_error_row(&sink, "dl:invalid", message).await;
                return;
            }
        };
        let entries = match self
            .downloads
            .list(
                filter,
                query.limit.min(MAX_DOWNLOAD_ENTRIES),
                cancel.clone(),
            )
            .await
        {
            Ok(entries) => entries,
            Err(DownloadsError::Cancelled) => return,
            Err(error) => {
                send_port_error(&sink, error).await;
                return;
            }
        };
        if cancel.is_cancelled() {
            return;
        }
        if entries.is_empty() {
            send_one(
                &sink,
                SearchItemDto {
                    id: "dl:empty".into(),
                    module_id: MODULE_ID.into(),
                    title: "No downloads found".into(),
                    subtitle: Some("The selected bounded filter returned no direct items".into()),
                    kind: "status".into(),
                    score: 0.0,
                    primary_action_id: "noop".into(),
                    primary_action_label: "OK".into(),
                    ..Default::default()
                },
            )
            .await;
            return;
        }
        let upserts = entries.into_iter().map(entry_item).collect();
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
        match result.kind.as_str() {
            "download" => vec![
                safe_action("open", "Open"),
                safe_action("reveal", "Reveal in Finder"),
                safe_action("copy_path", "Copy path"),
                ActionDescriptor {
                    id: ActionId::new("trash"),
                    label: "Move to Trash".into(),
                    risk: ActionRisk::Destructive,
                    confirmation: true,
                },
            ],
            "download_rename" => {
                let confirmation = result
                    .action_payload
                    .as_ref()
                    .and_then(|payload| payload.get("extension_change"))
                    .and_then(|value| value.as_bool())
                    .unwrap_or(true);
                vec![ActionDescriptor {
                    id: ActionId::new("rename"),
                    label: "Rename".into(),
                    risk: if confirmation {
                        ActionRisk::Confirm
                    } else {
                        ActionRisk::Safe
                    },
                    confirmation,
                }]
            }
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
            return invalid_action("missing Downloads action payload");
        };
        let Some(id) = payload.get("id").and_then(|value| value.as_str()) else {
            return invalid_action("missing download id");
        };
        let Some(expected_identity) = payload.get("identity").and_then(|value| value.as_str())
        else {
            return invalid_action("missing download identity");
        };

        // Every action re-reads the current entry. Mutations revalidate once more inside the port.
        let current = match self.downloads.resolve(id, cancel.clone()).await {
            Ok(entry) => entry,
            Err(error) => return port_outcome(error),
        };
        if current.identity != expected_identity {
            return port_outcome(DownloadsError::StaleIdentity);
        }
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }

        match request.action.id.as_str() {
            "open" => {
                match await_unless_cancelled(&cancel, self.opener.open(&current.path)).await {
                    None => ActionOutcome::Cancelled,
                    Some(Ok(())) => ActionOutcome::Success {
                        message: Some(format!("opened {}", current.display_name)),
                    },
                    Some(Err(error)) => ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: error.to_string(),
                            retryable: true,
                        },
                    },
                }
            }
            "reveal" => {
                match await_unless_cancelled(&cancel, self.opener.reveal(&current.path)).await {
                    None => ActionOutcome::Cancelled,
                    Some(Ok(())) => ActionOutcome::Success {
                        message: Some(format!("revealed {}", current.display_name)),
                    },
                    Some(Err(error)) => ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: error.to_string(),
                            retryable: true,
                        },
                    },
                }
            }
            "copy_path" => {
                let Some(path) = current.path.to_str() else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "path".into(),
                            message: "path is not valid UTF-8 and cannot be copied as text".into(),
                        },
                    };
                };
                match await_unless_cancelled(&cancel, self.pasteboard.write_text(path)).await {
                    None => ActionOutcome::Cancelled,
                    Some(Ok(())) => ActionOutcome::Success {
                        message: Some("copied download path".into()),
                    },
                    Some(Err(error)) => ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: error.to_string(),
                            retryable: true,
                        },
                    },
                }
            }
            "trash" => {
                if !request.confirmation {
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "confirmation required before moving a download to Trash"
                                .into(),
                        },
                    };
                }
                match self
                    .downloads
                    .trash(id, expected_identity, cancel.clone())
                    .await
                {
                    Ok(()) => ActionOutcome::Success {
                        message: Some(format!("moved {} to Trash", current.display_name)),
                    },
                    Err(error) => port_outcome(error),
                }
            }
            "rename" => {
                let Some(new_name) = payload.get("new_name").and_then(|value| value.as_str())
                else {
                    return invalid_action("missing new filename");
                };
                let extension_change = extension_changed(&current.display_name, new_name);
                if extension_change && !request.confirmation {
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "confirmation required when changing a file extension".into(),
                        },
                    };
                }
                if cancel.is_cancelled() {
                    return ActionOutcome::Cancelled;
                }
                match self
                    .downloads
                    .rename(id, expected_identity, new_name, cancel.clone())
                    .await
                {
                    Ok(renamed) => ActionOutcome::Success {
                        message: Some(format!("renamed to {}", renamed.display_name)),
                    },
                    Err(error) => port_outcome(error),
                }
            }
            _ => invalid_action("unknown Downloads action"),
        }
    }

    async fn teardown(&self) {}
}

impl DownloadsModule {
    async fn search_rename(&self, input: &str, sink: &SearchSink, cancel: CancellationToken) {
        let Some((id, new_name)) = input.split_once('|') else {
            send_error_row(
                sink,
                "dl:rename:invalid",
                "expected: /dl rename <result-id> | <new-name>",
            )
            .await;
            return;
        };
        let id = id.trim().strip_prefix("dl:").unwrap_or(id.trim());
        let new_name = new_name.trim();
        if id.is_empty() || new_name.is_empty() {
            send_error_row(
                sink,
                "dl:rename:invalid",
                "result id and new name are required",
            )
            .await;
            return;
        }
        let current = match self.downloads.resolve(id, cancel.clone()).await {
            Ok(entry) => entry,
            Err(DownloadsError::Cancelled) => return,
            Err(error) => {
                send_port_error(sink, error).await;
                return;
            }
        };
        if cancel.is_cancelled() {
            return;
        }
        let extension_change = extension_changed(&current.display_name, new_name);
        send_one(
            sink,
            SearchItemDto {
                id: format!("dl:rename:{}", current.id),
                module_id: MODULE_ID.into(),
                title: format!("Rename {} to {new_name}", current.display_name),
                subtitle: Some(if extension_change {
                    "The filename extension changes; confirmation is required".into()
                } else {
                    "The item will remain inside Downloads".into()
                }),
                kind: "download_rename".into(),
                score: 100.0,
                primary_action_id: "rename".into(),
                primary_action_label: "Rename".into(),
                primary_action_risk: if extension_change {
                    ActionRisk::Confirm
                } else {
                    ActionRisk::Safe
                },
                primary_action_confirmation: extension_change,
                action_payload: Some(serde_json::json!({
                    "id": current.id,
                    "identity": current.identity,
                    "new_name": new_name,
                    "extension_change": extension_change,
                })),
                ..Default::default()
            },
        )
        .await;
    }
}

fn strip_command<'a>(input: &'a str, command: &str) -> Option<&'a str> {
    let (word, rest) = input.split_once(char::is_whitespace).unwrap_or((input, ""));
    word.eq_ignore_ascii_case(command).then_some(rest.trim())
}

fn parse_filter(rest: &str) -> Result<DownloadsFilter, &'static str> {
    let normalized = rest.trim().to_lowercase();
    if normalized.is_empty() || normalized == "recent" {
        return Ok(DownloadsFilter::Recent);
    }
    if normalized == "large" {
        return Ok(DownloadsFilter::Large);
    }
    if let Some(days) = normalized.strip_prefix("old ") {
        let days = days
            .strip_suffix('d')
            .ok_or("old filter requires a duration such as 30d")?
            .parse::<u32>()
            .ok()
            .filter(|days| *days > 0 && *days <= 36_500)
            .ok_or("old duration must be between 1d and 36500d")?;
        return Ok(DownloadsFilter::Old { days });
    }
    if let Some(kind) = normalized.strip_prefix("type ") {
        let category = DownloadCategory::parse(kind)
            .ok_or("type must be archive, image, video, document, or installer")?;
        return Ok(DownloadsFilter::Type(category));
    }
    if normalized == "old" {
        return Err("expected: /dl old <days>d");
    }
    if normalized == "type" {
        return Err("expected: /dl type archive|image|video|document|installer");
    }
    Ok(DownloadsFilter::Text(rest.trim().into()))
}

fn entry_item(entry: DownloadEntry) -> SearchItemDto {
    let date = DateTime::<Utc>::from_timestamp(entry.modified_unix, 0)
        .map(|value| value.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "unknown date".into());
    let id = entry.id.clone();
    SearchItemDto {
        id: format!("dl:{id}"),
        module_id: MODULE_ID.into(),
        title: entry.display_name,
        subtitle: Some(format!(
            "{} · {} · {date} · id dl:{id}",
            if entry.is_directory {
                "folder"
            } else {
                entry.category.label()
            },
            format_bytes(entry.size_bytes),
        )),
        kind: "download".into(),
        score: 75.0,
        primary_action_id: "open".into(),
        primary_action_label: "Open".into(),
        secondary_actions: vec![
            action_dto("reveal", "Reveal in Finder", ActionRisk::Safe, false),
            action_dto("copy_path", "Copy path", ActionRisk::Safe, false),
            action_dto("trash", "Move to Trash", ActionRisk::Destructive, true),
        ],
        action_payload: Some(serde_json::json!({
            "id": entry.id,
            "identity": entry.identity,
        })),
        ..Default::default()
    }
}

fn action_dto(id: &str, label: &str, risk: ActionRisk, confirmation: bool) -> ActionDescriptorDto {
    ActionDescriptorDto {
        id: id.into(),
        label: label.into(),
        risk,
        confirmation,
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

fn extension_changed(old_name: &str, new_name: &str) -> bool {
    Path::new(old_name).extension() != Path::new(new_name).extension()
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1_024.0 && unit + 1 < UNITS.len() {
        value /= 1_024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
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

fn port_outcome(error: DownloadsError) -> ActionOutcome {
    if error == DownloadsError::Cancelled {
        return ActionOutcome::Cancelled;
    }
    ActionOutcome::Failed {
        kind: match error {
            DownloadsError::NotConfigured => FailureKind::NotConfigured {
                remediation: "Create or restore ~/Downloads".into(),
            },
            DownloadsError::Unavailable(reason) => FailureKind::Unavailable {
                reason,
                retryable: true,
            },
            DownloadsError::NotFound => FailureKind::NotFound {
                entity: "download item".into(),
            },
            DownloadsError::StaleIdentity => FailureKind::Conflict {
                reason: "download item changed; search again before acting".into(),
            },
            DownloadsError::Invalid(message) => FailureKind::InvalidInput {
                field: "download".into(),
                message,
            },
            DownloadsError::Conflict => FailureKind::Conflict {
                reason: "a Downloads item already uses that name".into(),
            },
            DownloadsError::Io(context) => FailureKind::Io { context },
            DownloadsError::Cancelled => unreachable!(),
        },
    }
}

async fn send_one(sink: &SearchSink, item: SearchItemDto) {
    let _ = sink
        .send(Event::ResultsChunk {
            request_id: String::new(),
            sequence: 1,
            upserts: vec![item],
            removed_ids: vec![],
        })
        .await;
}

async fn send_error_row(sink: &SearchSink, id: &str, message: &str) {
    send_one(
        sink,
        SearchItemDto {
            id: id.into(),
            module_id: MODULE_ID.into(),
            title: "Downloads command is invalid".into(),
            subtitle: Some(message.into()),
            kind: "command_error".into(),
            score: 0.0,
            primary_action_id: "noop".into(),
            primary_action_label: "Fix input".into(),
            ..Default::default()
        },
    )
    .await;
}

async fn send_port_error(sink: &SearchSink, error: DownloadsError) {
    let (id, title, subtitle, kind) = match error {
        DownloadsError::NotConfigured => (
            "dl:not-configured",
            "Downloads folder is not configured",
            "Create or restore ~/Downloads",
            "not_configured",
        ),
        DownloadsError::Unavailable(ref reason) => (
            "dl:unavailable",
            "Downloads folder is unavailable",
            reason.as_str(),
            "unavailable",
        ),
        DownloadsError::Io(ref reason) => (
            "dl:unavailable",
            "Downloads scan failed",
            reason.as_str(),
            "unavailable",
        ),
        DownloadsError::Cancelled => return,
        DownloadsError::NotFound => (
            "dl:not-found",
            "Download item no longer exists",
            "Search Downloads again before acting",
            "command_error",
        ),
        DownloadsError::StaleIdentity => (
            "dl:stale",
            "Download item changed",
            "Search Downloads again before acting",
            "command_error",
        ),
        DownloadsError::Invalid(ref reason) => (
            "dl:invalid",
            "Downloads operation is invalid",
            reason.as_str(),
            "command_error",
        ),
        DownloadsError::Conflict => (
            "dl:conflict",
            "A Downloads item already uses that name",
            "Choose a different filename",
            "command_error",
        ),
    };
    send_one(
        sink,
        SearchItemDto {
            id: id.into(),
            module_id: MODULE_ID.into(),
            title: title.into(),
            subtitle: Some(subtitle.into()),
            kind: kind.into(),
            score: 0.0,
            primary_action_id: "noop".into(),
            primary_action_label: "OK".into(),
            ..Default::default()
        },
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_application::{FakeDownloads, FakeOpenPath, FakePasteboard};
    use luma_domain::ResultId;
    use std::path::PathBuf;

    fn entry(name: &str, identity: &str) -> DownloadEntry {
        DownloadEntry {
            id: format!("id-{name}"),
            identity: identity.into(),
            display_name: name.into(),
            path: PathBuf::from("/fixture/Downloads").join(name),
            size_bytes: 42,
            modified_unix: 1_700_000_000,
            category: DownloadCategory::Document,
            is_directory: false,
        }
    }

    fn request(item: SearchItem, action: ActionDescriptor, confirmation: bool) -> ActionRequest {
        ActionRequest {
            result: item,
            action,
            confirmation,
        }
    }

    #[tokio::test]
    async fn list_uses_fake_and_exposes_bounded_actions() {
        let downloads = Arc::new(FakeDownloads::new(vec![entry("report.pdf", "v1")]));
        let module = DownloadsModule::with_deps(
            downloads,
            Arc::new(FakeOpenPath::new()),
            Arc::new(FakePasteboard::new()),
        );
        let (sink, mut receiver) = tokio::sync::mpsc::channel(2);
        module
            .search(
                Query::parse_with_prefixes_strict("/dl ", 50, |prefix| prefix == "dl"),
                sink,
                CancellationToken::new(),
            )
            .await;
        let Event::ResultsChunk { upserts, .. } = receiver.recv().await.unwrap() else {
            panic!("expected results");
        };
        assert_eq!(upserts.len(), 1);
        assert_eq!(upserts[0].title, "report.pdf");
        assert!(upserts[0]
            .secondary_actions
            .iter()
            .any(|action| action.id == "trash" && action.confirmation));
    }

    #[tokio::test]
    async fn stale_identity_blocks_open_before_fake_opener() {
        let downloads = Arc::new(FakeDownloads::new(vec![entry("report.pdf", "fresh")]));
        let opener = Arc::new(FakeOpenPath::new());
        let module =
            DownloadsModule::with_deps(downloads, opener.clone(), Arc::new(FakePasteboard::new()));
        let mut item = entry_item(entry("report.pdf", "stale")).into_domain();
        item.id = ResultId::new("dl:id-report.pdf");
        let outcome = module
            .perform(
                request(item, safe_action("open", "Open"), false),
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(
            outcome,
            ActionOutcome::Failed {
                kind: FailureKind::Conflict { .. }
            }
        ));
        assert_eq!(
            opener.open_count.load(std::sync::atomic::Ordering::SeqCst),
            0
        );
    }

    #[tokio::test]
    async fn trash_requires_confirmation_and_cancel_prevents_mutation() {
        let downloads = Arc::new(FakeDownloads::new(vec![entry("report.pdf", "v1")]));
        let module = DownloadsModule::with_deps(
            downloads.clone(),
            Arc::new(FakeOpenPath::new()),
            Arc::new(FakePasteboard::new()),
        );
        let item = entry_item(entry("report.pdf", "v1")).into_domain();
        let trash = ActionDescriptor {
            id: ActionId::new("trash"),
            label: "Move to Trash".into(),
            risk: ActionRisk::Destructive,
            confirmation: true,
        };
        let denied = module
            .perform(
                request(item.clone(), trash.clone(), false),
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(
            denied,
            ActionOutcome::Failed {
                kind: FailureKind::SecurityDenied { .. }
            }
        ));
        assert!(downloads.trash_calls.lock().unwrap().is_empty());

        let cancel = CancellationToken::new();
        cancel.cancel();
        assert_eq!(
            module.perform(request(item, trash, true), cancel).await,
            ActionOutcome::Cancelled
        );
        assert!(downloads.trash_calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn extension_change_is_rechecked_at_execution() {
        let downloads = Arc::new(FakeDownloads::new(vec![entry("report.pdf", "v1")]));
        let module = DownloadsModule::with_deps(
            downloads.clone(),
            Arc::new(FakeOpenPath::new()),
            Arc::new(FakePasteboard::new()),
        );
        let mut item = entry_item(entry("report.pdf", "v1")).into_domain();
        item.kind = "download_rename".into();
        item.action_payload.as_mut().unwrap()["new_name"] = serde_json::json!("report.txt");
        let rename = safe_action("rename", "Rename");
        let outcome = module
            .perform(request(item, rename, false), CancellationToken::new())
            .await;
        assert!(matches!(
            outcome,
            ActionOutcome::Failed {
                kind: FailureKind::SecurityDenied { .. }
            }
        ));
        assert!(downloads.rename_calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn not_configured_and_unavailable_are_distinct_rows() {
        for (error, expected_kind) in [
            (DownloadsError::NotConfigured, "not_configured"),
            (DownloadsError::Unavailable("fixture".into()), "unavailable"),
        ] {
            let downloads = Arc::new(FakeDownloads::new(vec![]));
            downloads.fail_with(error);
            let module = DownloadsModule::with_deps(
                downloads,
                Arc::new(FakeOpenPath::new()),
                Arc::new(FakePasteboard::new()),
            );
            let (sink, mut receiver) = tokio::sync::mpsc::channel(2);
            module
                .search(
                    Query::parse_with_prefixes_strict("/dl ", 50, |prefix| prefix == "dl"),
                    sink,
                    CancellationToken::new(),
                )
                .await;
            let Event::ResultsChunk { upserts, .. } = receiver.recv().await.unwrap() else {
                panic!("expected results");
            };
            assert_eq!(upserts[0].kind, expected_kind);
        }
    }
}
