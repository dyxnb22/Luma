import Foundation
import Testing

/// Wiring guards kept out of the main behavioral test pass.
/// These are smoke alarms for accidental regressions in cancel/hide wiring — not substitutes
/// for runtime integration tests in `LumaAppTests`.
@Test func launcherRootControllerDeclaresCancelPaths() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let path = root.appending(path: "Sources/LumaApp/Launcher/LauncherRootController.swift")
    let source = try String(contentsOf: path, encoding: .utf8)
    #expect(source.contains("func cancelLauncherAsyncWork()"))
    #expect(source.contains("func cancelActiveQueryAndSnapshotApply()"))
    #expect(!source.contains("func cancelAllLauncherWork()"))
    #expect(source.contains("workbenchPreviewTask"))
    #expect(source.contains("LauncherSnapshotApplyPipeline"))
}

@Test func launcherWindowControllerHideUsesCancelAllPath() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let path = root.appending(path: "Sources/LumaApp/Launcher/LauncherWindowController.swift")
    let source = try String(contentsOf: path, encoding: .utf8)
    #expect(source.contains("cancelActiveQueryAndSnapshotApply()"))
}
