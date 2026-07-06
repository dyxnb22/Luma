import Foundation
import Testing

@Test func finishHideUsesShowGenerationGuardInSource() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let path = root.appending(path: "Sources/LumaApp/Launcher/LauncherWindowController.swift")
    let source = try String(contentsOf: path, encoding: .utf8)
    #expect(source.contains("generationAtHide"))
    #expect(source.contains("guard showGeneration == generationAtHide"))
    #expect(source.contains("cancelPendingRestore()"))
}

@Test func restoreSessionUsesRestoreGenerationGuardInSource() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let path = root.appending(path: "Sources/LumaApp/Launcher/LauncherRootController.swift")
    let source = try String(contentsOf: path, encoding: .utf8)
    #expect(source.contains("restoreGeneration"))
    #expect(source.contains("guard self.restoreGeneration == generation"))
}
