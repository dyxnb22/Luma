import Foundation
import Testing
@testable import LumaApp

@Test @MainActor func menuBarShowWhileVisibleKeepsPanelVisible() async throws {
    let controller = LauncherWindowController()
    controller.showFromCarbonHotkey()
    #expect(controller.isPanelVisible)
    controller.showFromMenuBar()
    #expect(controller.isPanelVisible)
    controller.showFromMenuBar()
    #expect(controller.isPanelVisible)
}

@Test @MainActor func menuBarShowWhileVisibleDoesNotBlockSubsequentHide() async throws {
    let controller = LauncherWindowController()
    controller.showFromCarbonHotkey()
    controller.showFromMenuBar()
    #expect(controller.isPanelVisible)
    try await Task.sleep(for: .milliseconds(150))
    controller.hideFromVisibleHotkey()
    #expect(!controller.isPanelVisible)
}

@Test func menuBarRoutesThroughNamedShowEntry() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let coordinatorPath = root.appending(path: "Sources/LumaApp/App/AppCoordinator.swift")
    let source = try String(contentsOf: coordinatorPath, encoding: .utf8)
    #expect(source.contains("showFromMenuBar()"))
}
