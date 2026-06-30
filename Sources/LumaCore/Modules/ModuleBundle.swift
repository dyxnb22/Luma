import Foundation

/// Static registration surface for a built-in module: manifest, commands, warmup tier, and factory.
public protocol ModuleBundle {
    static var identifier: ModuleIdentifier { get }
    static var manifest: ModuleManifest { get }
    static var warmupTier: WarmupTier { get }
    static var commands: [CommandDefinition] { get }
    static var detailMetadata: FeatureCard? { get }
    static var presentation: ModulePresentation? { get }
    static func makeModule() -> any LumaModule
}

public extension ModuleBundle {
    static var identifier: ModuleIdentifier { manifest.identifier }

    static var detailMetadata: FeatureCard? { nil }

    static var presentation: ModulePresentation? { nil }

    /// When `defaultEnabled` is false, explains why the module is off by default.
    static var defaultOffNote: String? { nil }
}

public struct ModulePresentation: Sendable, Hashable {
    public let settingsSymbol: String
    public let listBadge: String?

    public init(settingsSymbol: String, listBadge: String? = nil) {
        self.settingsSymbol = settingsSymbol
        self.listBadge = listBadge
    }
}
