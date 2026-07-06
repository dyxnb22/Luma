import Foundation

public enum WarmupTier: Sendable, Equatable {
    /// Phase 1 startup; participates in global search when warm.
    case hotPath
    /// Warm on first targeted query, detail open, or explicit on-demand path.
    case onDemand
    /// Warm only when opening module detail.
    case detailOnly
}

public enum WarmupReason: Sendable, Equatable {
    case startup
    case query
    case detail
}

public enum WarmupState: Sendable, Equatable {
    case cold
    case warming
    case warm
    case tornDown
}

public enum WarmupPolicy: String, Sendable, Codable, CaseIterable {
    /// Warm pinned + hotPath modules at startup; others on demand.
    case eagerPinnedOnly
    /// Warm all enabled modules in background after Phase 1.
    case eagerAllEnabled
}

public enum ModuleWarmupDefaults {
    public static let defaultPinnedModuleIDs: Set<ModuleIdentifier> = [
        ModuleIdentifier(rawValue: "luma.apps"),
        ModuleIdentifier(rawValue: "luma.clipboard"),
        ModuleIdentifier(rawValue: "luma.snippets"),
        ModuleIdentifier(rawValue: "luma.secrets"),
        ModuleIdentifier(rawValue: "luma.todo"),
        ModuleIdentifier(rawValue: "luma.translate"),
        ModuleIdentifier(rawValue: "luma.wordbook"),
        ModuleIdentifier(rawValue: "luma.window-layouts"),
        ModuleIdentifier(rawValue: "luma.kill-process"),
        ModuleIdentifier(rawValue: "luma.quicklinks")
    ]
}

/// Global search dispatch tiers (fast modules return before deferred hot-path modules).
public enum GlobalSearchTiers {
    public static let fastModuleIDs: Set<ModuleIdentifier> = [
        ModuleIdentifier(rawValue: "luma.apps"),
        ModuleIdentifier(rawValue: "luma.quicklinks"),
        ModuleIdentifier(rawValue: "luma.kill-process"),
        ModuleIdentifier(rawValue: "luma.snippets")
    ]

    /// Modules that can return rows during global (non-prefixed) search.
    public static let contributingModuleIDs: Set<ModuleIdentifier> = [
        ModuleIdentifier(rawValue: "luma.apps"),
        ModuleIdentifier(rawValue: "luma.quicklinks"),
        ModuleIdentifier(rawValue: "luma.clipboard")
    ]
}
