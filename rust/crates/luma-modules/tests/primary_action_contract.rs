//! Primary-action contract coverage for manage/overwrite rows.

use async_trait::async_trait;
use luma_application::{
    AppEntry, AppsCatalogPort, FakeAccessibility, FakeOpenPath, FakeWindowCatalog, LumaModule,
    MemoryClipboardHistory, MemoryQuicklinksRepository, MemorySnippetsRepository,
    MemoryWordbookRepository, PasteboardError, PasteboardPort, QuicklinksRepository,
    SnippetsRepository, WarmupContext, WordContentInput, WordbookRepository,
};
use luma_domain::Query;
use luma_modules::{
    AppsModule, ClipboardModule, ClipboardSuppression, QuicklinksModule, SnippetsModule,
    WordbookModule,
};
use luma_test_support::assert_primary_actions_resolvable;
use std::path::PathBuf;
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
        Arc::new(FakeWindowCatalog::default()),
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
        Arc::new(FakeWindowCatalog::default()),
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

struct MemAppsCatalog {
    apps: Vec<AppEntry>,
}

#[async_trait]
impl AppsCatalogPort for MemAppsCatalog {
    async fn list_installed(&self) -> Result<Vec<AppEntry>, String> {
        Ok(self.apps.clone())
    }
    async fn launch(
        &self,
        _path: &std::path::Path,
    ) -> Result<(), luma_application::AppLaunchError> {
        Ok(())
    }
    async fn reveal(
        &self,
        _path: &std::path::Path,
    ) -> Result<(), luma_application::AppLaunchError> {
        Ok(())
    }
}

#[tokio::test]
async fn apps_search_row_matches_actions_contract() {
    let catalog = Arc::new(MemAppsCatalog {
        apps: vec![AppEntry {
            name: "Safari".into(),
            path: PathBuf::from("/Applications/Safari.app"),
            bundle_id: None,
        }],
    });
    let m = AppsModule::new(catalog, Arc::new(MemPb(Mutex::new(None))));
    m.warmup(WarmupContext {
        cancel: CancellationToken::new(),
    })
    .await;
    assert_primary_actions_resolvable(&m, Query::parse("app safari", 20)).await;
    m.teardown().await;
}

#[tokio::test]
async fn secrets_vault_row_matches_actions_contract() {
    use luma_application::FakeKeychain;
    use luma_modules::SecretsModule;

    let keychain = Arc::new(FakeKeychain {
        unlocked: true,
        entries: tokio::sync::Mutex::new(std::collections::BTreeMap::from([(
            "api-token".into(),
            "super-secret".into(),
        )])),
    });
    let m = SecretsModule::with_deps(
        keychain,
        Arc::new(MemPb(Mutex::new(None))),
        Arc::new(ClipboardSuppression::new()),
    );
    m.warmup(WarmupContext {
        cancel: CancellationToken::new(),
    })
    .await;
    assert_primary_actions_resolvable(&m, Query::parse("sec ", 20)).await;
    m.teardown().await;
}

#[tokio::test]
async fn windows_row_matches_actions_contract() {
    use luma_application::{FakeWindowCatalog, WindowEntry};
    use luma_modules::WindowsModule;

    let catalog = Arc::new(FakeWindowCatalog::with_entries(
        vec![WindowEntry {
            id: "pid:1|num:1".into(),
            app_name: "Cursor".into(),
            app_bundle_id: None,
            title: "Luma".into(),
            is_on_screen: true,
            layer: 0,
            owner_pid: 1,
        }],
        Some("Cursor".into()),
    ));
    let m = WindowsModule::with_catalog(catalog);
    m.warmup(WarmupContext {
        cancel: CancellationToken::new(),
    })
    .await;
    assert_primary_actions_resolvable(&m, Query::parse("win luma", 20)).await;
    m.teardown().await;
}

#[tokio::test]
async fn notes_daily_row_matches_actions_contract() {
    use luma_modules::NotesModule;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let m = NotesModule::with_root_for_tests(Some(dir.path().to_path_buf()));
    m.warmup(WarmupContext {
        cancel: CancellationToken::new(),
    })
    .await;
    assert_primary_actions_resolvable(&m, Query::parse("n daily", 20)).await;
    m.teardown().await;
}

#[tokio::test]
async fn wordbook_due_and_add_rows_match_actions_contract() {
    let store = Arc::new(MemoryWordbookRepository::new());
    store
        .upsert_content(&WordContentInput {
            term: "latency".into(),
            phonetic: "".into(),
            meaning: "延迟".into(),
            example: "".into(),
            category: "".into(),
        })
        .unwrap();
    let id = store.get_by_term("latency").unwrap().unwrap().id;
    store.review(id, "known").unwrap();
    // Force due by setting next_review_at in the past via review (memory uses now).
    // list_due filters next_review_at <= now; memory review sets next to now so it may be due.
    let m = WordbookModule::with_store_for_tests(store.clone(), Arc::new(MemPb(Mutex::new(None))));
    m.warmup(WarmupContext {
        cancel: CancellationToken::new(),
    })
    .await;
    assert_primary_actions_resolvable(&m, Query::parse("wb due", 20)).await;
    assert_primary_actions_resolvable(&m, Query::parse("wb add latency | 延迟 new | example", 20))
        .await;
    let items = luma_test_support::collect_search_items(
        &m,
        Query::parse("wb add latency | 延迟 new | example", 20),
    )
    .await;
    assert!(
        items.iter().any(|i| {
            i.id.as_str() == "wb:add:latency"
                && i.kind == "update"
                && i.primary_action.id.as_str() == "add"
                && i.primary_action.confirmation
        }),
        "expected overwrite add row, got: {:?}",
        items
            .iter()
            .map(|i| (i.id.as_str().to_string(), i.kind.clone()))
            .collect::<Vec<_>>()
    );
    m.teardown().await;
}
