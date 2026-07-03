import LumaCore

/// Visual metadata for in-panel module detail headers under Route C.
/// Prefer `ModuleRegistry.moduleDetailMetadata()` for new code; this type remains for existing call sites.
public enum FeatureCatalog {
    /// Visual metadata (gradients, triggers) for in-panel module detail headers under Route C.
    public static func moduleDetailMetadata() -> [FeatureCard] {
        ModuleRegistry.moduleDetailMetadata()
    }
}
