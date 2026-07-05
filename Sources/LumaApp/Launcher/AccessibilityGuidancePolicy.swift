import Foundation
import LumaCore
import LumaModules
import LumaServices

struct AccessibilityGuidanceContext: Sendable, Equatable {
    enum Surface: Sendable, Equatable {
        case none
        case targetedModule(ModuleIdentifier)
        case moduleDetail(ModuleIdentifier)
        case openAppsWindowControlUsed
    }

    let surface: Surface
}

enum AccessibilityGuidancePolicy {
    private static let guidanceModuleIDs: Set<ModuleIdentifier> = [
        .snippets,
        .windowLayouts,
        .menuItems
    ]

    static func shouldShowBanner(
        context: AccessibilityGuidanceContext,
        enabledModules: Set<ModuleIdentifier>
    ) -> Bool {
        guard AXService.isProcessTrusted() == false else { return false }

        switch context.surface {
        case .none:
            return false
        case .targetedModule(let module), .moduleDetail(let module):
            return guidanceModuleIDs.contains(module) && enabledModules.contains(module)
        case .openAppsWindowControlUsed:
            return enabledModules.contains(.apps) || guidanceModuleIDs.contains(where: enabledModules.contains)
        }
    }

    static func isGuidanceModule(_ module: ModuleIdentifier) -> Bool {
        guidanceModuleIDs.contains(module)
    }
}
