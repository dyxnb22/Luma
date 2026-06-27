import Foundation
import LumaCore
import LumaInfrastructure
import LumaServices
import Testing
@testable import LumaModules

private func sampleApp(
    name: String,
    bundleID: String,
    path: String,
    localizedName: String? = nil,
    aliases: [String] = [],
    pinyinFull: String = "",
    pinyinInitials: String = ""
) -> AppRecord {
    AppRecord(
        bundleID: bundleID,
        url: URL(fileURLWithPath: path),
        name: name,
        localizedName: localizedName ?? name,
        aliases: aliases,
        pinyinFull: pinyinFull,
        pinyinInitials: pinyinInitials
    )
}

@Test func appIndexFindsPrefixAndSubstringMatches() {
    let index = AppIndex(apps: [
        sampleApp(name: "Safari", bundleID: "com.apple.Safari", path: "/Applications/Safari.app"),
        sampleApp(name: "System Settings", bundleID: "com.apple.systempreferences", path: "/System/Applications/System Settings.app"),
        sampleApp(name: "Visual Studio Code", bundleID: "com.microsoft.VSCode", path: "/Applications/Visual Studio Code.app", aliases: ["vscode", "vsc"], pinyinInitials: "vsc")
    ])

    #expect(index.search("saf").first?.name == "Safari")
    #expect(index.search("code").first?.name == "Visual Studio Code")
    #expect(index.search("vsc").first?.name == "Visual Studio Code")
}

@Test func appIndexHandlesEmptyAndMissingQueries() {
    let index = AppIndex(apps: [
        sampleApp(name: "A", bundleID: "a", path: "/A.app"),
        sampleApp(name: "B", bundleID: "b", path: "/B.app")
    ])

    #expect(index.search("").count == 2)
    #expect(index.search("missing").isEmpty)
}

@Test func appIndexMatchesChineseAliasAndPinyin() {
    let index = AppIndex(apps: [
        sampleApp(
            name: "WeChat",
            bundleID: "com.tencent.xinWeChat",
            path: "/Applications/WeChat.app",
            aliases: ["微信", "wechat", "wx"],
            pinyinFull: "wei xin",
            pinyinInitials: "wx"
        )
    ])

    #expect(index.search("微信").first?.bundleID == "com.tencent.xinWeChat")
    #expect(index.search("wechat").first?.bundleID == "com.tencent.xinWeChat")
    #expect(index.search("wx").first?.bundleID == "com.tencent.xinWeChat")
}

@Test func appIndexSubsequenceFuzzyMatch() {
    let index = AppIndex(apps: [
        sampleApp(name: "Visual Studio Code", bundleID: "com.microsoft.VSCode", path: "/Applications/Visual Studio Code.app", pinyinInitials: "vsc")
    ])
    #expect(index.search("vsc").first?.name == "Visual Studio Code")
}

@Test func appIndexMatchesCaseInsensitiveBundleID() {
    let index = AppIndex(apps: [
        sampleApp(name: "Visual Studio Code", bundleID: "com.microsoft.VSCode", path: "/Applications/Visual Studio Code.app")
    ])

    #expect(index.search("VSCODE").first?.name == "Visual Studio Code")
}

@Test func appsModuleReturnsLaunchActionForUserSearch() async {
    let module = AppsModule()
    let context = QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(40)))
    let result = await module.handle(Query(raw: "safari", sequence: 1), context: context)
    #expect(result.items.allSatisfy { item in
        if case .launchApp = item.primaryAction.kind { return true }
        return false
    })
}

@Test func appsModuleSearchIncludesSecondaryActions() async {
    let module = AppsModule()
    await module.warmup(testAppsModuleContext())
    let context = QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(40)))
    let result = await module.handle(Query(raw: "app safari", sequence: 1), context: context)
    guard let item = result.items.first else {
        Issue.record("Expected at least one app result")
        return
    }
    #expect(!item.secondaryActions.isEmpty)
    #expect(item.secondaryActions.contains { $0.title == "Reveal in Finder" })
    #expect(item.secondaryActions.contains { $0.title == "Copy App Path" })
}

private func testAppsModuleContext() -> ModuleContext {
    ModuleContext(
        logger: LumaLogger(),
        metrics: LumaMetrics(),
        database: ApplicationSupportPaths(),
        pasteboard: PasteboardService(),
        accessibility: AXService(),
        fileSystem: FSEventsService(),
        translation: TranslationService(),
        config: ConfigurationStore()
    )
}
