use super::*;
use std::time::Instant;

pub(crate) const MAX_CANCEL_INTENTS: usize = 64;
pub(crate) const CANCEL_INTENT_TTL: Duration = Duration::from_secs(10);

impl EngineInner {
    pub(crate) fn prune_cancel_intents(&mut self, now: Instant) {
        self.cancel_intents
            .retain(|_, inserted| now.duration_since(*inserted) < CANCEL_INTENT_TTL);
        while self.cancel_intents.len() > MAX_CANCEL_INTENTS {
            let oldest = self
                .cancel_intents
                .iter()
                .min_by_key(|(_, inserted)| *inserted)
                .map(|(id, _)| id.clone());
            let Some(id) = oldest else {
                break;
            };
            self.cancel_intents.remove(&id);
        }
    }

    pub(crate) fn record_cancel_intent(&mut self, request_id: String) {
        let now = Instant::now();
        self.prune_cancel_intents(now);
        self.cancel_intents.insert(request_id, now);
        while self.cancel_intents.len() > MAX_CANCEL_INTENTS {
            let oldest = self
                .cancel_intents
                .iter()
                .min_by_key(|(_, inserted)| *inserted)
                .map(|(id, _)| id.clone());
            let Some(id) = oldest else {
                break;
            };
            self.cancel_intents.remove(&id);
        }
    }

    /// Returns true when a non-expired cancel intent was present and consumed.
    pub(crate) fn take_cancel_intent(&mut self, request_id: &str) -> bool {
        let now = Instant::now();
        self.prune_cancel_intents(now);
        self.cancel_intents.remove(request_id).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

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
            operation_order: VecDeque::new(),
            next_operation_generation: 0,
            latest_preview_id: 0,
            results_by_id: HashMap::new(),
            result_order: VecDeque::new(),
            module_states: HashMap::new(),
        }
    }

    #[test]
    fn cancel_intents_are_capped() {
        let mut inner = test_inner();
        for i in 0..MAX_CANCEL_INTENTS + 5 {
            inner.record_cancel_intent(format!("req-{i}"));
        }
        assert!(inner.cancel_intents.len() <= MAX_CANCEL_INTENTS);
    }

    #[test]
    fn expired_cancel_intent_is_pruned() {
        let mut inner = test_inner();
        let stale = Instant::now() - CANCEL_INTENT_TTL - Duration::from_secs(1);
        inner.cancel_intents.insert("stale".into(), stale);
        inner.record_cancel_intent("fresh".into());
        assert!(!inner.cancel_intents.contains_key("stale"));
        assert!(inner.cancel_intents.contains_key("fresh"));
    }
}
