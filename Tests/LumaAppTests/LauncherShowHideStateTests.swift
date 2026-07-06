import Foundation
import LumaCore
import Testing

@Test func launcherWindowControllerWiresVisibilitySession() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let path = root.appending(path: "Sources/LumaApp/Launcher/LauncherWindowController.swift")
    let source = try String(contentsOf: path, encoding: .utf8)
    #expect(source.contains("LauncherPanelVisibilitySession"))
    #expect(source.contains("shouldCompleteHide(generationAtHide:"))
    #expect(source.contains("shouldCompleteDeferredShow(generation:"))
    #expect(source.contains("cancelPanelHideAnimation()"))
    #expect(source.contains("panel.animations.removeAll()"))
    #expect(source.contains("panelHideTask"))
    #expect(source.contains("cancelActiveQueryAndSnapshotApply()"))
}

@Test func launcherRootControllerWiresRestoreCancellationGeneration() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let path = root.appending(path: "Sources/LumaApp/Launcher/LauncherRootController.swift")
    let source = try String(contentsOf: path, encoding: .utf8)
    #expect(source.contains("CancellationGeneration"))
    #expect(source.contains("restoreGeneration.isCurrent(generation)"))
}
