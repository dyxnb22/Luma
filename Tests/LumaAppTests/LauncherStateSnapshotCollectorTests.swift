import Foundation
import Testing

@Test func launcherStateSnapshotCollectorReferencesRequiredFields() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let collectorPath = root.appending(path: "Sources/LumaApp/Launcher/LauncherStateSnapshotCollector.swift")
    let exporterPath = root.appending(path: "Sources/LumaApp/Infrastructure/LauncherStateSnapshotExporter.swift")
    let corePath = root.appending(path: "Sources/LumaCore/Launcher/LauncherStateSnapshot.swift")

    let collector = try String(contentsOf: collectorPath, encoding: .utf8)
    let exporter = try String(contentsOf: exporterPath, encoding: .utf8)
    let core = try String(contentsOf: corePath, encoding: .utf8)

    #expect(collector.contains("LauncherStateSnapshotCollector"))
    #expect(collector.contains("isDetailModeActive"))
    #expect(collector.contains("splitRightPane"))
    #expect(collector.contains("detailContainerHidden"))
    #expect(exporter.contains("launcher-state.json"))
    #expect(exporter.contains("launcher-state-violations.json"))
    #expect(core.contains("LauncherStateInvariantChecker"))
}

@Test func launcherRootControllerWiresStateSnapshotExport() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let path = root.appending(path: "Sources/LumaApp/Launcher/LauncherRootController.swift")
    let source = try String(contentsOf: path, encoding: .utf8)
    #expect(source.contains("collectStateSnapshot"))
    #expect(source.contains("reconcileLauncherStateAfterShow"))
    #expect(source.contains("detailContextActive: searchBar.isDetailModeActive"))
    #expect(source.contains("scheduleStateSnapshotExport"))
}
