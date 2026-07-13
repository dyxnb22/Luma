//! Short-lived suppression of sensitive pasteboard values from clipboard history.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Process-local salted hashes of values that must not enter clipboard history.
#[derive(Debug, Default)]
pub struct ClipboardSuppression {
    leases: Mutex<HashMap<u64, Instant>>,
}

impl ClipboardSuppression {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn suppress(&self, text: &str, ttl: Duration) {
        let hash = hash_text(text);
        let expires = Instant::now() + ttl;
        if let Ok(mut leases) = self.leases.lock() {
            Self::gc(&mut leases);
            leases.insert(hash, expires);
        }
    }

    pub fn is_suppressed(&self, text: &str) -> bool {
        let hash = hash_text(text);
        let Ok(mut leases) = self.leases.lock() else {
            return false;
        };
        Self::gc(&mut leases);
        leases.contains_key(&hash)
    }

    fn gc(leases: &mut HashMap<u64, Instant>) {
        let now = Instant::now();
        leases.retain(|_, exp| *exp > now);
    }
}

fn hash_text(text: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    "luma.clipboard.suppress.v1".hash(&mut hasher);
    text.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suppress_matches_exact_value() {
        let s = ClipboardSuppression::new();
        s.suppress("tok_abc123XYZ", Duration::from_secs(30));
        assert!(s.is_suppressed("tok_abc123XYZ"));
        assert!(!s.is_suppressed("other"));
    }
}
