use super::*;

pub(crate) const MAX_ENGINE_RESULTS: usize = 512;

impl EngineInner {
    pub(crate) fn clear_results(&mut self) {
        self.results_by_id.clear();
        self.result_order.clear();
    }

    pub(crate) fn remove_result(&mut self, id: &str) -> bool {
        if self.results_by_id.remove(id).is_some() {
            if let Some(pos) = self.result_order.iter().position(|existing| existing == id) {
                self.result_order.remove(pos);
            }
            true
        } else {
            false
        }
    }

    pub(crate) fn evict_overflow_results(&mut self) -> Vec<String> {
        let mut removed = Vec::new();
        while self.results_by_id.len() > MAX_ENGINE_RESULTS {
            let Some(oldest) = self.result_order.pop_front() else {
                break;
            };
            if self.results_by_id.remove(&oldest).is_some() {
                removed.push(oldest);
            }
        }
        removed
    }

    /// Insert or update a cached result. New ids are tracked in insertion order; updates do not
    /// affect order. Returns ids evicted because the cache exceeded [`MAX_ENGINE_RESULTS`].
    pub(crate) fn insert_result(
        &mut self,
        id: String,
        item: luma_domain::SearchItem,
    ) -> Vec<String> {
        let is_new = !self.results_by_id.contains_key(&id);
        self.results_by_id.insert(id.clone(), item);
        if is_new {
            self.result_order.push_back(id);
        }
        self.evict_overflow_results()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ModuleId, ResultId, SearchItem};

    fn sample_item(id: &str) -> SearchItem {
        SearchItem {
            id: ResultId::new(id),
            module_id: ModuleId::new("luma.test"),
            title: id.into(),
            subtitle: None,
            kind: "test".into(),
            score: 1.0,
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

    fn test_inner() -> EngineInner {
        let (tx, _) = tokio::sync::broadcast::channel(8);
        EngineInner {
            registry: ModuleRegistry::new(),
            event_broadcast_tx: tx,
            session_cancel: CancellationToken::new(),
            searches: HashMap::new(),
            cancel_intents: HashMap::new(),
            pending_searches: HashMap::new(),
            operations: HashMap::new(),
            results_by_id: HashMap::new(),
            result_order: VecDeque::new(),
            module_states: HashMap::new(),
        }
    }

    #[test]
    fn insert_evicts_oldest_beyond_cap() {
        let mut inner = test_inner();
        for i in 0..MAX_ENGINE_RESULTS + 10 {
            let id = format!("r{i}");
            inner.insert_result(id, sample_item(&format!("r{i}")));
        }
        assert_eq!(inner.results_by_id.len(), MAX_ENGINE_RESULTS);
        assert_eq!(inner.result_order.len(), MAX_ENGINE_RESULTS);
        assert!(!inner.results_by_id.contains_key("r0"));
        assert!(!inner.results_by_id.contains_key("r9"));
        assert!(inner.results_by_id.contains_key("r10"));
        assert!(inner
            .results_by_id
            .contains_key(&format!("r{}", MAX_ENGINE_RESULTS + 9)));
    }

    #[test]
    fn update_existing_does_not_reorder() {
        let mut inner = test_inner();
        inner.insert_result("a".into(), sample_item("a"));
        inner.insert_result("b".into(), sample_item("b"));
        inner.insert_result("a".into(), sample_item("a2"));
        assert_eq!(inner.result_order.len(), 2);
        assert_eq!(inner.result_order.front().map(String::as_str), Some("a"));
    }

    #[test]
    fn remove_result_syncs_order_queue() {
        let mut inner = test_inner();
        inner.insert_result("a".into(), sample_item("a"));
        inner.insert_result("b".into(), sample_item("b"));
        assert!(inner.remove_result("a"));
        assert_eq!(inner.result_order.len(), 1);
        assert_eq!(inner.result_order[0], "b");
        assert!(!inner.remove_result("missing"));
    }

    #[test]
    fn clear_results_empties_order_queue() {
        let mut inner = test_inner();
        inner.insert_result("a".into(), sample_item("a"));
        inner.clear_results();
        assert!(inner.results_by_id.is_empty());
        assert!(inner.result_order.is_empty());
    }
}
