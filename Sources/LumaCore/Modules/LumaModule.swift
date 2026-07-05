import Foundation
import CoreGraphics

public struct ModuleIdentifier: Hashable, Sendable, RawRepresentable, Codable {
    public let rawValue: String

    public init(rawValue: String) {
        self.rawValue = rawValue
    }
}

public struct ModuleCapabilities: OptionSet, Sendable {
    public let rawValue: Int

    public init(rawValue: Int) {
        self.rawValue = rawValue
    }

    public static let queryable = ModuleCapabilities(rawValue: 1 << 0)
    public static let providesActions = ModuleCapabilities(rawValue: 1 << 1)
    public static let backgroundUpdater = ModuleCapabilities(rawValue: 1 << 2)
}

public struct ModuleManifest: Sendable {
    public let identifier: ModuleIdentifier
    public let displayName: String
    public let capabilities: ModuleCapabilities
    public let defaultEnabled: Bool
    public let priority: Int
    public let queryTimeout: Duration

    public init(
        identifier: ModuleIdentifier,
        displayName: String,
        capabilities: ModuleCapabilities,
        defaultEnabled: Bool,
        priority: Int,
        queryTimeout: Duration
    ) {
        self.identifier = identifier
        self.displayName = displayName
        self.capabilities = capabilities
        self.defaultEnabled = defaultEnabled
        self.priority = priority
        self.queryTimeout = queryTimeout
    }
}

public protocol LumaModule: Sendable {
    static var manifest: ModuleManifest { get }

    func warmup(_ context: ModuleContext) async
    func handle(_ query: Query, context: QueryContext) async -> ModuleResult
    func perform(_ action: Action, context: ActionContext) async throws
    func teardown() async
}

public extension LumaModule {
    func warmup(_ context: ModuleContext) async {}

    func perform(_ action: Action, context: ActionContext) async throws {
        throw ModuleError.unsupportedAction(action.id)
    }

    func teardown() async {}
}

public struct ModuleResult: Sendable {
    public var items: [ResultItem]
    public var isPartial: Bool
    public var diagnostic: ModuleDiagnostic?

    public init(items: [ResultItem], isPartial: Bool = false, diagnostic: ModuleDiagnostic? = nil) {
        self.items = items
        self.isPartial = isPartial
        self.diagnostic = diagnostic
    }

    public static func empty(for id: ModuleIdentifier, diagnostic: ModuleDiagnostic? = nil) -> ModuleResult {
        ModuleResult(items: [], diagnostic: diagnostic)
    }
}

public struct ModuleDiagnostic: Sendable {
    public enum Kind: Sendable {
        case timeout
        case error
        case degraded
        case permissionRequired
    }

    public let kind: Kind
    public let message: String

    public init(kind: Kind, message: String) {
        self.kind = kind
        self.message = message
    }
}

public enum ModuleError: Error, Sendable {
    case unsupportedAction(ActionID)
    case dataUnavailable
    case actionTimedOut
    case permissionRequired(Permission)
}

public enum Permission: Sendable {
    case accessibility
    case automation
}
