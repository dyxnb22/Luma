import LumaCore

public enum FeatureCatalog {
    /// Visual metadata (gradients, triggers) for in-panel module detail headers under Route C.
    public static func moduleDetailMetadata() -> [FeatureCard] {
        ModuleRegistry.moduleDetailMetadata()
    }
}
