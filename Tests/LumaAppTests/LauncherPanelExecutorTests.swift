import Foundation
import Testing

@Test func launcherPanelExecutorContractInSource() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let panelPath = root.appending(path: "Sources/LumaApp/Launcher/LauncherPanel.swift")
    let source = try String(contentsOf: panelPath, encoding: .utf8)
    #expect(!source.contains("@MainActor\nfinal class LauncherPanel"))
    #expect(source.contains("nonisolated override func cancelOperation"))
    #expect(source.contains("nonisolated override func performKeyEquivalent"))
    #expect(source.contains("nonisolated override func keyDown"))
    #expect(source.contains("guard isVisible else { return false }"))
}

@Test func launcherHintBarLayoutIsNonisolatedInSource() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let path = root.appending(path: "Sources/LumaApp/Launcher/LauncherHintBar.swift")
    let source = try String(contentsOf: path, encoding: .utf8)
    #expect(source.contains("@preconcurrency import AppKit"))
    #expect(source.contains("nonisolated override func layout()"))
    #expect(!source.contains("@MainActor\nfinal class LauncherHintBar"))
}
