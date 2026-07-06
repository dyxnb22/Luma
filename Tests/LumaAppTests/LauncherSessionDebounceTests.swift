import Foundation
import LumaCore
import LumaInfrastructure
import Testing
@testable import LumaApp

@Test @MainActor func saveSearchQueryDebouncesSessionPersistWrites() async {
    LauncherPerfCounters.reset()
    let defaults = UserDefaults(suiteName: "LumaAppTests.sessionDebounce")!
    defaults.removePersistentDomain(forName: "LumaAppTests.sessionDebounce")
    let store = LauncherSessionStore()
    store.sessionConfigOverride = ConfigurationStore(defaults: defaults)
    store.suppressPersistence = false

    for index in 0..<100 {
        store.saveSearchQuery("query-\(index)")
    }

    try? await Task.sleep(for: .milliseconds(50))
    #expect(LauncherPerfCounters.count(for: .sessionPersist) == 0)

    try? await Task.sleep(for: .milliseconds(450))
    #expect(LauncherPerfCounters.count(for: .sessionPersist) == 1)

    store.flushPendingWrites()
    try? await Task.sleep(for: .milliseconds(50))
    #expect(LauncherPerfCounters.count(for: .sessionPersist) == 1)
}
