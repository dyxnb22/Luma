import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules

@MainActor
final class LauncherSessionStore {
    var suppressPersistence = false

    func saveCurrentSession(
        moduleID: ModuleIdentifier?,
        query: String,
        translateContent: (source: String, output: String)?
    ) {
        guard !suppressPersistence, let config = ModuleDetailRegistry.config else { return }
        Task {
            await config.setLauncherLastModuleID(moduleID?.rawValue)
            await config.setLauncherLastQuery(query)
            if let translateContent {
                await config.setLauncherTranslateSourceText(translateContent.source)
                await config.setLauncherTranslateOutputText(translateContent.output)
            }
        }
    }

    func saveHomeSession(query: String) {
        guard !suppressPersistence, let config = ModuleDetailRegistry.config else { return }
        Task {
            await config.setLauncherLastModuleID(nil)
            await config.setLauncherLastQuery(query)
        }
    }

    func saveSearchQuery(_ query: String) {
        guard !suppressPersistence, let config = ModuleDetailRegistry.config else { return }
        Task { await config.setLauncherLastQuery(query) }
    }

    enum RestoreDecision {
        case openModule(ModuleIdentifier, translateSource: String, translateOutput: String)
        case restoreQuery(String)
        case showHome
    }

    func restoreDecision(
        moduleRaw: String?,
        query: String,
        translateSource: String,
        translateOutput: String
    ) -> RestoreDecision {
        if let moduleRaw,
           ModuleDetailRegistry.make(for: ModuleIdentifier(rawValue: moduleRaw)) != nil {
            return .openModule(
                ModuleIdentifier(rawValue: moduleRaw),
                translateSource: translateSource,
                translateOutput: translateOutput
            )
        }
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty { return .showHome }
        return .restoreQuery(query)
    }

    func loadPersistedSession() async -> (moduleRaw: String?, query: String, translateSource: String, translateOutput: String) {
        guard let config = ModuleDetailRegistry.config else {
            return (nil, "", "", "")
        }
        let moduleRaw = await config.launcherLastModuleID()
        let query = await config.launcherLastQuery()
        let translateSource = await config.launcherTranslateSourceText()
        let translateOutput = await config.launcherTranslateOutputText()
        return (moduleRaw, query, translateSource, translateOutput)
    }
}
