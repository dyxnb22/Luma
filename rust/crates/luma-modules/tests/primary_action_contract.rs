//! Primary-action contract coverage for manage/overwrite rows.

use async_trait::async_trait;
use luma_application::{
    FakeAccessibility, FakeOpenPath, LumaModule, MemoryClipboardHistory,
    MemoryQuicklinksRepository, MemorySnippetsRepository, PasteboardError, PasteboardPort,
    QuicklinksRepository, SnippetsRepository, WarmupContext,
};
use luma_domain::Query;
use luma_modules::{ClipboardModule, ClipboardSuppression, QuicklinksModule, SnippetsModule};
use luma_test_support::assert_primary_actions_resolvable;
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
async fn clipboard_clear_row_matches_actions_contract() {
    let m = ClipboardModule::with_deps(
        Arc::new(MemoryClipboardHistory::new()),
        Arc::new(MemPb(Mutex::new(None))),
        Arc::new(FakeAccessibility {
            trusted: false,
            paste_ok: false,
        }),
        Arc::new(ClipboardSuppression::new()),
    );
    m.warmup(WarmupContext {
        cancel: CancellationToken::new(),
    })
    .await;
    assert_primary_actions_resolvable(&m, Query::parse("clip clear", 20)).await;
    m.teardown().await;
}

#[tokio::test]
async fn quicklinks_overwrite_row_matches_actions_contract() {
    let store = Arc::new(MemoryQuicklinksRepository::new());
    store.upsert("docs", "https://example.com").unwrap();
    let m = QuicklinksModule::with_deps(
        store,
        Arc::new(FakeOpenPath::new()),
        Arc::new(MemPb(Mutex::new(None))),
    );
    m.warmup(WarmupContext {
        cancel: CancellationToken::new(),
    })
    .await;
    assert_primary_actions_resolvable(&m, Query::parse("ql add docs https://example.com/new", 20))
        .await;
    let items = luma_test_support::collect_search_items(
        &m,
        Query::parse("ql add docs https://example.com/new", 20),
    )
    .await;
    assert!(
        items.iter().any(|i| {
            i.id.as_str() == "ql:add:docs"
                && i.kind == "update"
                && i.primary_action.id.as_str() == "add"
                && i.primary_action.confirmation
        }),
        "expected ql:add:docs overwrite row, got: {:?}",
        items
            .iter()
            .map(|i| {
                (
                    i.id.as_str().to_string(),
                    i.kind.clone(),
                    i.primary_action.id.as_str().to_string(),
                    i.primary_action.confirmation,
                )
            })
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn snippets_overwrite_row_matches_actions_contract() {
    let store = Arc::new(MemorySnippetsRepository::new());
    store.upsert("sig", "old body").unwrap();
    let m = SnippetsModule::with_store(
        store,
        Arc::new(MemPb(Mutex::new(None))),
        Arc::new(FakeAccessibility {
            trusted: false,
            paste_ok: false,
        }),
    );
    m.warmup(WarmupContext {
        cancel: CancellationToken::new(),
    })
    .await;
    assert_primary_actions_resolvable(&m, Query::parse("snip add sig new body text", 20)).await;
    let items =
        luma_test_support::collect_search_items(&m, Query::parse("snip add sig new body text", 20))
            .await;
    assert!(
        items.iter().any(|i| {
            i.id.as_str() == "snip:add:sig"
                && i.primary_action.id.as_str() == "add"
                && i.primary_action.confirmation
        }),
        "expected snip:add:sig overwrite row, got: {:?}",
        items
            .iter()
            .map(|i| {
                (
                    i.id.as_str().to_string(),
                    i.kind.clone(),
                    i.primary_action.id.as_str().to_string(),
                    i.primary_action.confirmation,
                )
            })
            .collect::<Vec<_>>()
    );
}
