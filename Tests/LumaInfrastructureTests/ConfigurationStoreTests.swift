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
    await store.setClipboardHistoryEnabled(false)
    await store.setClipboardIgnoredBundleIDs(["com.test.app"])
    await store.setClipboardPasteBehavior("copyOnly")
    await store.setTranslationTargetLanguage("zh-Hans")
    await store.setSecretsAutoClearSeconds(15)
    await store.setSecretsRelockTimeoutSeconds(120)
    await store.setSecretsRequireUnlockOnLaunch(false)

    #expect(await store.clipboardMaxEntries() == 42)
    #expect(await store.clipboardMaxAgeDays() == 3)
    #expect(await store.clipboardMaxEntrySizeKB() == 12)
    #expect(await store.clipboardHistoryEnabled() == false)
    #expect(await store.clipboardIgnoredBundleIDs() == ["com.test.app"])
    #expect(await store.clipboardPasteBehavior() == "copyOnly")
    #expect(await store.translationTargetLanguage() == "zh-Hans")
    #expect(await store.secretsAutoClearSeconds() == 15)
    #expect(await store.secretsRelockTimeoutSeconds() == 120)
    #expect(await store.secretsRequireUnlockOnLaunch() == false)
}
