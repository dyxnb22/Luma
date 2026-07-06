import Foundation
import LumaCore

/// Applies reducer effects from `LauncherSessionState.apply`.
@MainActor
struct LauncherSessionEffectApplier {
    struct Environment {
        var cancelAllTasks: () -> Void
        var clearDetailModeState: () -> Void
    }

    static func apply(_ effects: [LauncherSessionEffect], environment: Environment) {
        for effect in effects {
            switch effect {
            case .cancelAllTasks:
                environment.cancelAllTasks()
            case .clearDetailModeState:
                environment.clearDetailModeState()
            }
        }
    }
}
