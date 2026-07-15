//! Runtime capability probes for module preflight.

/// Platform capabilities required by module manifests (`required_capabilities`).
pub trait CapabilityPort: Send + Sync {
    fn has(&self, capability: &str) -> bool;
}

/// Deterministic probe set for unit tests.
#[derive(Clone, Debug, Default)]
pub struct FakeCapabilities {
    pub accessibility: bool,
    pub keychain: bool,
}

impl CapabilityPort for FakeCapabilities {
    fn has(&self, capability: &str) -> bool {
        match capability {
            "accessibility" => self.accessibility,
            "keychain" => self.keychain,
            _ => true,
        }
    }
}
