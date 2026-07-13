use luma_protocol::{Event, SearchItemDto};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// In-process mock engine for Phase 1. No real modules / DB / FFI.
#[derive(Clone, Default)]
pub struct MockEngine {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Default)]
struct Inner {
    next_token: u64,
    searches: HashMap<String, u64>,
}

impl MockEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_search(
        &self,
        request_id: String,
        query: String,
        tx: mpsc::Sender<Event>,
    ) -> JoinHandle<()> {
        let token = {
            let mut g = self.inner.lock().expect("mock engine lock");
            g.next_token += 1;
            let token = g.next_token;
            g.searches.insert(request_id.clone(), token);
            token
        };

        let engine = self.clone();
        tokio::spawn(async move {
            let _ = tx
                .send(Event::SearchStarted {
                    request_id: request_id.clone(),
                })
                .await;
            let _ = tx
                .send(Event::ResultsReset {
                    request_id: request_id.clone(),
                })
                .await;

            // Simulate streaming without blocking the TUI event loop (this is a task).
            for (seq, title) in mock_hits(&query).into_iter().enumerate() {
                if !engine.is_active(&request_id, token) {
                    let _ = tx
                        .send(Event::SearchCancelled {
                            request_id: request_id.clone(),
                        })
                        .await;
                    return;
                }
                let _ = tx
                    .send(Event::ResultsChunk {
                        request_id: request_id.clone(),
                        sequence: (seq as u64) + 1,
                        upserts: vec![SearchItemDto {
                            id: format!("mock-{seq}"),
                            module_id: "mock".into(),
                            title,
                            subtitle: Some(format!("query={query}")),
                            kind: "mock".into(),
                            score: 100.0 - seq as f64,
                            primary_action_id: "open".into(),
                            primary_action_label: "Open".into(),
                        }],
                        removed_ids: vec![],
                    })
                    .await;
                tokio::task::yield_now().await;
            }

            if engine.is_active(&request_id, token) {
                let total = mock_hits(&query).len();
                let _ = tx
                    .send(Event::SearchFinished {
                        request_id,
                        total,
                        elapsed_ms: 1,
                    })
                    .await;
            }
        })
    }

    pub fn cancel(&self, request_id: &str) {
        let mut g = self.inner.lock().expect("mock engine lock");
        g.searches.remove(request_id);
    }

    fn is_active(&self, request_id: &str, token: u64) -> bool {
        let g = self.inner.lock().expect("mock engine lock");
        g.searches.get(request_id).copied() == Some(token)
    }
}

fn mock_hits(query: &str) -> Vec<String> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    vec![
        format!("{q} — Mock Result A"),
        format!("{q} — Mock Result B"),
        format!("{q} — Mock Result C"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn cancel_prevents_finished_for_old_token() {
        let engine = MockEngine::new();
        let (tx, mut rx) = mpsc::channel::<Event>(32);
        let handle = engine.start_search("r1".into(), "abc".into(), tx);
        engine.cancel("r1");
        handle.await.unwrap();

        let mut saw_cancelled = false;
        let mut saw_finished = false;
        while let Ok(ev) = rx.try_recv() {
            match ev {
                Event::SearchCancelled { .. } => saw_cancelled = true,
                Event::SearchFinished { .. } => saw_finished = true,
                _ => {}
            }
        }
        assert!(saw_cancelled || !saw_finished);
        assert!(!saw_finished);
    }
}
