//! Storage health probes injected at composition time (Doctor).

/// Probes LumaNext stores without the engine opening storage types directly.
pub trait StorageProbePort: Send + Sync {
    fn probe_stores(&self, settings_ok: bool) -> serde_json::Value;
}
