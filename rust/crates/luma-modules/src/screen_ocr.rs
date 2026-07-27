use crate::cancel::await_unless_cancelled;
use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, LumaModule, ModuleManifest, ModuleState, PasteboardPort,
    ScreenOcrError, ScreenOcrPort, SearchMode, SearchSink, WarmupContext, MAX_OCR_TEXT_BYTES,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{Event, SearchItemDto};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

const MODULE_ID: &str = "luma.ocr";

pub struct ScreenOcrModule {
    manifest: ModuleManifest,
    ocr: Arc<dyn ScreenOcrPort>,
    pasteboard: Arc<dyn PasteboardPort>,
}

impl ScreenOcrModule {
    pub fn with_deps(ocr: Arc<dyn ScreenOcrPort>, pasteboard: Arc<dyn PasteboardPort>) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new(MODULE_ID),
                display_name: "Screen OCR".into(),
                triggers: vec!["ocr".into()],
                default_enabled: true,
                search_mode: SearchMode::TargetedOnly,
                // Permission is checked only when /ocr executes; warmup never prompts or disables.
                required_capabilities: vec![],
                workbench: luma_application::WorkbenchMeta {
                    glyph: Some("O".into()),
                    suggested_query: Some("/ocr ".into()),
                    empty_hint: Some("/ocr · select a screen region and copy text".into()),
                    supports_browse: false,
                    commands: vec![crate::ux::command_spec(
                        "/ocr",
                        "Select a screen region and copy locally recognized text",
                        "/ocr ",
                        None,
                    )],
                },
            },
            ocr,
            pasteboard,
        }
    }
}

#[async_trait]
impl LumaModule for ScreenOcrModule {
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
        if !query.rest_normalized().is_empty() {
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![crate::ux::command_error(
                        MODULE_ID,
                        "ocr:arguments-invalid",
                        "Screen OCR takes no arguments",
                        "Usage: /ocr",
                    )],
                    removed_ids: vec![],
                })
                .await;
            return;
        }
        let _ = sink
            .send(Event::ResultsChunk {
                request_id: String::new(),
                sequence: 1,
                upserts: vec![SearchItemDto {
                    id: "ocr:capture".into(),
                    module_id: MODULE_ID.into(),
                    title: "Select screen region and copy recognized text".into(),
                    subtitle: Some(
                        "Apple Vision · local only · screenshot deleted after recognition".into(),
                    ),
                    kind: "screen_ocr".into(),
                    score: 100.0,
                    primary_action_id: "capture_copy".into(),
                    primary_action_label: "Select region".into(),
                    ..Default::default()
                }],
                removed_ids: vec![],
            })
            .await;
    }

    async fn actions(&self, result: &SearchItem) -> Vec<ActionDescriptor> {
        if result.kind == "screen_ocr" {
            vec![safe_action("capture_copy", "Select region")]
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
        if request.action.id.as_str() != "capture_copy" {
            return ActionOutcome::Failed {
                kind: FailureKind::InvalidInput {
                    field: "action".into(),
                    message: "unsupported Screen OCR action".into(),
                },
            };
        }
        let text = match self.ocr.recognize_region(cancel.clone()).await {
            Ok(text) => bounded_text(&text),
            Err(error) => return ocr_outcome(error),
        };
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        if text.is_empty() {
            return ocr_outcome(ScreenOcrError::Empty);
        }
        match await_unless_cancelled(&cancel, self.pasteboard.write_text(&text)).await {
            None => ActionOutcome::Cancelled,
            Some(Ok(())) => ActionOutcome::Success {
                // Never put recognized text or the temporary path in outcomes/Recall.
                message: Some("recognized text copied".into()),
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

fn bounded_text(value: &str) -> String {
    let normalized = value.replace("\r\n", "\n").replace('\r', "\n");
    let trimmed = normalized.trim();
    if trimmed.len() <= MAX_OCR_TEXT_BYTES {
        return trimmed.into();
    }
    let mut end = MAX_OCR_TEXT_BYTES;
    while end > 0 && !trimmed.is_char_boundary(end) {
        end -= 1;
    }
    trimmed[..end].trim_end().into()
}

fn safe_action(id: &str, label: &str) -> ActionDescriptor {
    ActionDescriptor {
        id: ActionId::new(id),
        label: label.into(),
        risk: ActionRisk::Safe,
        confirmation: false,
    }
}

fn ocr_outcome(error: ScreenOcrError) -> ActionOutcome {
    match error {
        ScreenOcrError::Cancelled => ActionOutcome::Cancelled,
        ScreenOcrError::PermissionRequired => ActionOutcome::Failed {
            kind: FailureKind::PermissionRequired {
                capability: "screen_recording".into(),
                guidance:
                    "Allow Luma in System Settings → Privacy & Security → Screen & System Audio Recording, then retry /ocr"
                        .into(),
            },
        },
        ScreenOcrError::Empty => ActionOutcome::Failed {
            kind: FailureKind::NotFound {
                entity: "recognized text".into(),
            },
        },
        ScreenOcrError::CaptureUnavailable(reason)
        | ScreenOcrError::RecognitionUnavailable(reason) => ActionOutcome::Failed {
            kind: FailureKind::Unavailable {
                reason,
                retryable: true,
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_application::{FakePasteboard, FakeScreenOcr};
    use luma_domain::ResultId;
    use luma_test_support::collect_search_items;

    fn result() -> SearchItem {
        SearchItem {
            id: ResultId::new("ocr:capture"),
            module_id: ModuleId::new(MODULE_ID),
            title: "Select region".into(),
            subtitle: None,
            kind: "screen_ocr".into(),
            score: 1.0,
            primary_action: safe_action("capture_copy", "Select region"),
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: None,
        }
    }

    fn request() -> ActionRequest {
        ActionRequest {
            result: result(),
            action: safe_action("capture_copy", "Select region"),
            confirmation: false,
        }
    }

    #[tokio::test]
    async fn arguments_are_rejected_before_capture_is_offered() {
        let module = ScreenOcrModule::with_deps(
            Arc::new(FakeScreenOcr::new([])),
            Arc::new(FakePasteboard::new()),
        );
        let items = collect_search_items(&module, Query::parse("/ocr now", 20)).await;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, "command_error");
        assert_ne!(items[0].primary_action.id.as_str(), "capture_copy");
    }

    #[tokio::test]
    async fn recognized_text_is_bounded_then_copied_without_echoing_it() {
        let secret_marker = "PRIVATE_OCR_TEXT";
        let oversized = format!("{secret_marker}{}", "界".repeat(100_000));
        let ocr = Arc::new(FakeScreenOcr::new([Ok(oversized)]));
        let pasteboard = Arc::new(FakePasteboard::new());
        let module = ScreenOcrModule::with_deps(ocr, pasteboard.clone());
        let outcome = module.perform(request(), CancellationToken::new()).await;
        assert_eq!(
            outcome,
            ActionOutcome::Success {
                message: Some("recognized text copied".into())
            }
        );
        let copied = pasteboard.last_text().unwrap();
        assert!(copied.len() <= MAX_OCR_TEXT_BYTES);
        assert!(copied.starts_with(secret_marker));
        assert!(!format!("{outcome:?}").contains(secret_marker));
    }

    #[tokio::test]
    async fn permission_cancel_empty_and_unavailable_are_structured() {
        let cases = [
            (ScreenOcrError::PermissionRequired, "permission_required"),
            (ScreenOcrError::Cancelled, "cancelled"),
            (ScreenOcrError::Empty, "not_found"),
            (
                ScreenOcrError::CaptureUnavailable("capture failed".into()),
                "unavailable",
            ),
            (
                ScreenOcrError::RecognitionUnavailable("Vision failed".into()),
                "unavailable",
            ),
        ];
        for (error, expected) in cases {
            let module = ScreenOcrModule::with_deps(
                Arc::new(FakeScreenOcr::new([Err(error)])),
                Arc::new(FakePasteboard::new()),
            );
            let outcome = module.perform(request(), CancellationToken::new()).await;
            let actual = match outcome {
                ActionOutcome::Cancelled => "cancelled",
                ActionOutcome::Failed {
                    kind: FailureKind::PermissionRequired { .. },
                } => "permission_required",
                ActionOutcome::Failed {
                    kind: FailureKind::NotFound { .. },
                } => "not_found",
                ActionOutcome::Failed {
                    kind: FailureKind::Unavailable { .. },
                } => "unavailable",
                _ => "unexpected",
            };
            assert_eq!(actual, expected);
        }
    }

    #[tokio::test]
    async fn cancellation_and_pasteboard_failure_never_report_success() {
        let ocr = Arc::new(FakeScreenOcr::new([Ok("text".into())]));
        let module = ScreenOcrModule::with_deps(ocr.clone(), Arc::new(FakePasteboard::new()));
        let cancel = CancellationToken::new();
        cancel.cancel();
        assert_eq!(
            module.perform(request(), cancel).await,
            ActionOutcome::Cancelled
        );
        assert_eq!(*ocr.calls.lock().unwrap(), 0);

        struct FailPasteboard;
        #[async_trait]
        impl PasteboardPort for FailPasteboard {
            async fn read_text(&self) -> Result<Option<String>, luma_application::PasteboardError> {
                Ok(None)
            }

            async fn write_text(
                &self,
                _text: &str,
            ) -> Result<(), luma_application::PasteboardError> {
                Err(luma_application::PasteboardError::Unavailable(
                    "fixture failure".into(),
                ))
            }
        }
        let failed = ScreenOcrModule::with_deps(
            Arc::new(FakeScreenOcr::new([Ok("text".into())])),
            Arc::new(FailPasteboard),
        )
        .perform(request(), CancellationToken::new())
        .await;
        assert!(matches!(
            failed,
            ActionOutcome::Failed {
                kind: FailureKind::Unavailable { .. }
            }
        ));
    }

    #[test]
    fn required_capabilities_remain_empty() {
        let module = ScreenOcrModule::with_deps(
            Arc::new(FakeScreenOcr::new([])),
            Arc::new(FakePasteboard::new()),
        );
        assert!(module.manifest().required_capabilities.is_empty());
    }
}
