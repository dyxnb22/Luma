import Foundation
import LumaCore

@MainActor
final class ModuleDetailReloadRouter {
    private var handlers: [ModuleIdentifier: () -> Void] = [:]

    func register(_ moduleID: ModuleIdentifier, handler: @escaping () -> Void) {
        handlers[moduleID] = handler
    }

    func unregister(_ moduleID: ModuleIdentifier) {
        handlers.removeValue(forKey: moduleID)
    }

    func reload(_ moduleID: ModuleIdentifier) {
        handlers[moduleID]?()
    }
}
