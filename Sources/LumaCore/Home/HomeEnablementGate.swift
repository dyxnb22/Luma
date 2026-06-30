import Foundation

/// Shared enabled-module set for home providers that are not contextual contributors.
public final class HomeEnablementGate: @unchecked Sendable {
    private let lock = NSLock()
    private var enabledModuleIDs: Set<ModuleIdentifier>?

    public init() {}

    public func update(_ ids: Set<ModuleIdentifier>) {
        lock.lock()
        enabledModuleIDs = ids
        lock.unlock()
    }

    public func contains(_ id: ModuleIdentifier) -> Bool {
        lock.lock()
        defer { lock.unlock() }
        guard let enabledModuleIDs else { return true }
        return enabledModuleIDs.contains(id)
    }

    public func snapshot() -> Set<ModuleIdentifier>? {
        lock.lock()
        defer { lock.unlock() }
        return enabledModuleIDs
    }
}
