import Foundation
import Testing
import LumaCore
@testable import LumaModules

@Test func appIndexFindsPrefixAndSubstringMatches() {
    let index = AppIndex(apps: [
        AppRecord(name: "Safari", bundleID: "com.apple.Safari", url: URL(fileURLWithPath: "/Applications/Safari.app")),
        AppRecord(name: "System Settings", bundleID: "com.apple.systempreferences", url: URL(fileURLWithPath: "/System/Applications/System Settings.app")),
        AppRecord(name: "Visual Studio Code", bundleID: "com.microsoft.VSCode", url: URL(fileURLWithPath: "/Applications/Visual Studio Code.app"))
    ])

    #expect(index.search("saf").first?.name == "Safari")
    #expect(index.search("code").first?.name == "Visual Studio Code")
}

@Test func appIndexHandlesEmptyAndMissingQueries() {
    let index = AppIndex(apps: [
        AppRecord(name: "A", bundleID: "a", url: URL(fileURLWithPath: "/A.app")),
        AppRecord(name: "B", bundleID: "b", url: URL(fileURLWithPath: "/B.app"))
    ])

    #expect(index.search("").count == 2)
    #expect(index.search("missing").isEmpty)
}

@Test func appIndexMatchesCaseInsensitiveBundleID() {
    let index = AppIndex(apps: [
        AppRecord(name: "Visual Studio Code", bundleID: "com.microsoft.VSCode", url: URL(fileURLWithPath: "/Applications/Visual Studio Code.app"))
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
