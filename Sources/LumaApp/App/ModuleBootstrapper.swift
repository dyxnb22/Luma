import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices

/// Registers built-in modules and runs startup warmup policy.
@MainActor
enum ModuleBootstrapper {
    static func registerAndWarmup(
        host: ModuleHost,
        config: ConfigurationStore,
        modules: [any LumaModule],
        processMemorySampler: ProcessMemorySampler,
        onModulesReady: @MainActor @escaping () -> Void,
        onMemoryPressureReady: @MainActor @escaping () -> Void
    ) async {
        await config.migrateIfNeeded()
        for module in modules {
            await host.register(module)
        }
        let enabled = await config.enabledModules()
        let pinned = await config.pinnedModuleIDs()
        let policy = await config.warmupPolicy()
        await host.configureWarmupPolicy(pinned: pinned)
        await host.configureGlobalSearchModuleIDs(ModuleRegistry.globalSearchModuleIDs)
        await host.applyEnabledSet(enabled)

        let startupWarm = pinned.intersection(enabled ?? Set(modules.map { type(of: $0).manifest.identifier }))
        await host.warmupIfNeeded(ids: startupWarm, reason: .startup)
        onModulesReady()
        Task {
            await processMemorySampler.start()
        }

        if policy == .eagerAllEnabled {
            await host.warmupRemainingEnabled()
        }
        onMemoryPressureReady()
    }
}
