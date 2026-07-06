import Foundation
import LumaCore
import LumaInfrastructure

@MainActor
final class LauncherSessionStore {
    var suppressPersistence = false
    /// Test-only override when `LauncherEnvironment.current` is unset.
    var sessionConfigOverride: ConfigurationStore?

    private var pendingSearchQuery: String?
    private var searchQuerySaveTask: Task<Void, Never>?
    private var pendingResumeState: LauncherResumeState?
    private var resumeSaveTask: Task<Void, Never>?
    private static let searchQueryDebounce: Duration = .milliseconds(400)
    private static let resumeDebounce: Duration = .milliseconds(500)

    func saveCurrentSession(
        moduleID: ModuleIdentifier?,
        query: String,
        translateContent: (source: String, output: String)?
    ) {
        guard !suppressPersistence, let config = LauncherEnvironment.current?.config else { return }
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
        guard !suppressPersistence, let config = LauncherEnvironment.current?.config else { return }
        Task {
            await config.setLauncherLastModuleID(nil)
            await config.setLauncherLastQuery(query)
        }
    }

    func saveSearchQuery(_ query: String) {
        guard !suppressPersistence else { return }
        pendingSearchQuery = query
        searchQuerySaveTask?.cancel()
        searchQuerySaveTask = Task { @MainActor [weak self] in
            try? await Task.sleep(for: Self.searchQueryDebounce)
            guard !Task.isCancelled, let self else { return }
            await self.flushSearchQueryIfNeeded()
        }
    }

    func scheduleResumeSave(_ state: LauncherResumeState) {
        guard !suppressPersistence else { return }
        pendingResumeState = state
        resumeSaveTask?.cancel()
        resumeSaveTask = Task { @MainActor [weak self] in
            try? await Task.sleep(for: Self.resumeDebounce)
            guard !Task.isCancelled, let self else { return }
            self.flushResumeStateIfNeeded()
        }
    }

    func flushPendingWrites() {
        searchQuerySaveTask?.cancel()
        searchQuerySaveTask = nil
        resumeSaveTask?.cancel()
        resumeSaveTask = nil
        Task { @MainActor in
            await flushSearchQueryIfNeeded()
            flushResumeStateIfNeeded()
        }
    }

    private var activeConfig: ConfigurationStore? {
        sessionConfigOverride ?? LauncherEnvironment.current?.config
    }

    private func flushSearchQueryIfNeeded() async {
        guard !suppressPersistence,
              let query = pendingSearchQuery,
              let config = activeConfig else { return }
        pendingSearchQuery = nil
        LauncherPerfCounters.increment(.sessionPersist)
        await config.setLauncherLastQuery(query)
    }

    private func flushResumeStateIfNeeded() {
        guard !suppressPersistence, let state = pendingResumeState else { return }
        pendingResumeState = nil
        LauncherPerfCounters.increment(.sessionPersist)
        Task.detached(priority: .utility) {
            LauncherResumeStore.save(state)
        }
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
           let environment = LauncherEnvironment.current,
           environment.canMakeDetailView(for: ModuleIdentifier(rawValue: moduleRaw)) {
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
        guard let config = LauncherEnvironment.current?.config else {
            return (nil, "", "", "")
        }
        let moduleRaw = await config.launcherLastModuleID()
        let query = await config.launcherLastQuery()
        let translateSource = await config.launcherTranslateSourceText()
        let translateOutput = await config.launcherTranslateOutputText()
        return (moduleRaw, query, translateSource, translateOutput)
    }
}
