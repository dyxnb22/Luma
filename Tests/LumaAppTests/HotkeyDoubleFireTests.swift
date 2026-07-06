import Foundation
import Testing
@testable import LumaApp

@Test @MainActor func carbonHotkeyShowOnlyWhenHidden() async throws {
    let controller = LauncherWindowController()
    #expect(controller.isPanelVisible == false)
    controller.showFromCarbonHotkey()
    #expect(controller.isPanelVisible == true)
    controller.showFromCarbonHotkey()
    #expect(controller.isPanelVisible == true)
}

@Test @MainActor func visibleHotkeyHidesOnce() async throws {
    let controller = LauncherWindowController()
    controller.showFromCarbonHotkey()
    #expect(controller.isPanelVisible == true)
    try await Task.sleep(for: .milliseconds(150))
    controller.hideFromVisibleHotkey()
    #expect(controller.isPanelVisible == false)
}

@Test @MainActor func rapidDoubleToggleDebounces() async throws {
    let controller = LauncherWindowController()
    controller.toggle()
    #expect(controller.isPanelVisible == true)
    try await Task.sleep(for: .milliseconds(150))
    controller.toggle()
    #expect(controller.isPanelVisible == false)
}

@Test func appCoordinatorCarbonPathDoesNotToggleWhenVisible() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let path = root.appending(path: "Sources/LumaApp/App/AppCoordinator.swift")
    let source = try String(contentsOf: path, encoding: .utf8)
    #expect(source.contains("showFromCarbonHotkey"))
}
