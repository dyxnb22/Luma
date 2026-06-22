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

@Test func configurationStorePersistsRuntimeSettings() async {
    let suite = "luma.tests.\(UUID().uuidString)"
    let defaults = UserDefaults(suiteName: suite)!
    defer { UserDefaults().removePersistentDomain(forName: suite) }
    let store = ConfigurationStore(defaults: defaults)

    await store.setClipboardMaxEntries(42)
    await store.setClipboardMaxAgeDays(3)
    await store.setClipboardMaxEntrySizeKB(12)
    await store.setTranslationTargetLanguage("zh-Hans")

    #expect(await store.clipboardMaxEntries() == 42)
    #expect(await store.clipboardMaxAgeDays() == 3)
    #expect(await store.clipboardMaxEntrySizeKB() == 12)
    #expect(await store.translationTargetLanguage() == "zh-Hans")
}
