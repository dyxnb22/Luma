import Foundation
import Testing

@Test func launcherRootControllerWiresHomeSplitPlanner() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let path = root.appending(path: "Sources/LumaApp/Launcher/LauncherRootController.swift")
    let source = try String(contentsOf: path, encoding: .utf8)
    #expect(source.contains("LauncherHomeSplitPlanner.layout"))
    #expect(source.contains("currentHomeSplitState()"))
}
