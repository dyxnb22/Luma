import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules

/// Settings window callbacks decoupled from `AppCoordinator`.
@MainActor
struct SettingsCoordinator {
    let onModulesChanged: @MainActor @Sendable (Set<ModuleIdentifier>) -> Void
    let onPinnedChanged: @MainActor @Sendable (Set<ModuleIdentifier>) -> Void
    let onClipboardSettingsChanged: @MainActor @Sendable (SettingsSnapshot) -> Void
    let onSecretsSettingsChanged: @MainActor @Sendable (Int, Int) -> Void
    let onLatencyHUDChanged: @MainActor @Sendable (Bool) -> Void

    func makeWindowController(config: ConfigurationStore, usage: PersistentUsageTracker) -> SettingsWindowController {
        SettingsWindowController(
            config: config,
            usage: usage,
            onModulesChanged: onModulesChanged,
            onPinnedChanged: onPinnedChanged,
            onClipboardSettingsChanged: onClipboardSettingsChanged,
            onSecretsSettingsChanged: onSecretsSettingsChanged,
            onLatencyHUDChanged: onLatencyHUDChanged
        )
    }
}
