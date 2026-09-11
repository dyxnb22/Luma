//! Paste-target contract: Snippets fail honestly without a target.

use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, FakeAccessibility, FakeWindowCatalog, LumaModule,
    MemorySnippetsRepository, PasteboardError, PasteboardPort, SnippetsRepository,
};
use luma_domain::{ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, SearchItem};
use luma_modules::SnippetsModule;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

struct MemPb(Mutex<Option<String>>);

#[async_trait]
impl PasteboardPort for MemPb {
    async fn read_text(&self) -> Result<Option<String>, PasteboardError> {
        Ok(self.0.lock().await.clone())
    }
    async fn write_text(&self, text: &str) -> Result<(), PasteboardError> {
        *self.0.lock().await = Some(text.into());
        Ok(())
    }
}

#[tokio::test]
async fn snippets_paste_without_target_fails_unavailable() {
    let store = Arc::new(MemorySnippetsRepository::new());
    store.upsert("sig", "hello body").unwrap();
    let catalog = Arc::new(FakeWindowCatalog::default());
    let m = SnippetsModule::with_store(
        store,
        Arc::new(MemPb(Mutex::new(None))),
        Arc::new(FakeAccessibility::new(true, true)),
        catalog.clone(),
    );
    m.warmup(luma_application::WarmupContext {
        cancel: CancellationToken::new(),
    })
    .await;
    let outcome = m
        .perform(
            ActionRequest {
                result: SearchItem {
                    id: luma_domain::ResultId::new("snip:sig"),
                    module_id: ModuleId::new("luma.snippets"),
                    title: "sig".into(),
                    subtitle: None,
                    kind: "snippet".into(),
                    score: 1.0,
                    primary_action: ActionDescriptor {
                        id: ActionId::new("paste"),
                        label: "Paste".into(),
                        risk: ActionRisk::Confirm,
                        confirmation: false,
                    },
                    secondary_actions: vec![],
                    ui_intent: None,
                    action_payload: None,
                },
                action: ActionDescriptor {
                    id: ActionId::new("paste"),
                    label: "Paste".into(),
                    risk: ActionRisk::Confirm,
                    confirmation: false,
                },
                confirmation: false,
            },
            CancellationToken::new(),
        )
        .await;
    match outcome {
        ActionOutcome::Failed {
            kind: FailureKind::Unavailable { reason, .. },
        } => {
            assert!(reason.contains("no paste target"), "{reason}");
        }
        other => panic!("expected Unavailable, got {other:?}"),
    }
    assert!(catalog.focus_app_calls.lock().await.is_empty());
}
