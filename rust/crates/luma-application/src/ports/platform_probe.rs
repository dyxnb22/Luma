//! Platform permission probes injected at composition time (Doctor).

/// Probes Accessibility / window listing without the engine importing platform crates.
pub trait PlatformProbePort: Send + Sync {
    /// Returns JSON with `accessibility`, `probes`, and `ax_trusted` keys.
    fn probe_platform(&self) -> serde_json::Value;
}

/// Deterministic fake for engine / doctor tests.
pub struct FakePlatformProbe {
    pub value: serde_json::Value,
}

impl PlatformProbePort for FakePlatformProbe {
    fn probe_platform(&self) -> serde_json::Value {
        self.value.clone()
    }
}
