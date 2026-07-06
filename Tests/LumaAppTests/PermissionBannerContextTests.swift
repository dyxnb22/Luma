import Foundation
import LumaCore
import Testing
@testable import LumaApp

@Test func accessibilityGuidancePolicyTargetsMenuItemsModule() {
    #expect(AccessibilityGuidancePolicy.isGuidanceModule(.menuItems))
    #expect(!AccessibilityGuidancePolicy.isGuidanceModule(.apps))
}

@Test func permissionBannerRoutesFromCurrentSearchValue() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let path = root.appending(path: "Sources/LumaApp/Launcher/LauncherRootController.swift")
    let source = try String(contentsOf: path, encoding: .utf8)
    #expect(source.contains("searchBar.stringValue"))
    #expect(source.contains("viewModel.commandRouter.route(raw: searchBar.stringValue)"))
    #expect(!source.contains("lastNormalizedQueryState"))
}
