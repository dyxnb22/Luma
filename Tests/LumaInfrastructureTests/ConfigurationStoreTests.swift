import Foundation
import Testing
import LumaCore
@testable import LumaInfrastructure

@Test func configurationStorePersistsEnabledModules() async {
    let suite = "luma.tests.\(UUID().uuidString)"
    let defaults = UserDefaults(suiteName: suite)!
    defer { UserDefaults().removePersistentDomain(forName: suite) }
    let store = ConfigurationStore(defaults: defaults)
    let ids: Set<ModuleIdentifier> = [ModuleIdentifier(rawValue: "luma.apps")]
    await store.setEnabledModules(ids)
    #expect(await store.enabledModules() == ids)
}
