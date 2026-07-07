import Foundation
import Testing

// Wiring-only guards: helper names and call sites. Not behavioral regression tests.
// Detail exit ordering and panel hide cleanup are not exercised here.

@Test func detailExitChromeRoutesThroughPlannerOutcome() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let path = root.appending(path: "Sources/LumaApp/Launcher/LauncherRootController.swift")
    let source = try String(contentsOf: path, encoding: .utf8)
    #expect(source.contains("func applyDetailExitFromChrome()"))
    #expect(source.contains("func applyDetailExitOutcome(_ outcome: LauncherDetailExitOutcome)"))
    #expect(source.contains("exitDetailFromChrome() {\n        applyDetailExitFromChrome()"))
    #expect(source.contains("dismissDetailForNewQuery()"))
    #expect(source.contains("persistDetailForActionDismiss()"))
}

@Test func panelHideFinalizationSharedBetweenFadeAndActionDismiss() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let path = root.appending(path: "Sources/LumaApp/Launcher/LauncherWindowController.swift")
    let source = try String(contentsOf: path, encoding: .utf8)
    #expect(source.contains("func finalizePanelHidden()"))
    #expect(source.contains("finalizePanelHidden()"))
    #expect(source.contains("prepareDetailForHide()"))
}
