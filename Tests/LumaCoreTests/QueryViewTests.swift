import Foundation
import Testing

@Test func queryViewFileReplacesNormalizedQueryState() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let queryViewPath = root.appending(path: "Sources/LumaApp/Launcher/QueryView.swift")
    let source = try String(contentsOf: queryViewPath, encoding: .utf8)
    #expect(source.contains("struct QueryView"))
    #expect(source.contains("typealias NormalizedQueryState = QueryView"))
    let normalizedPath = root.appending(path: "Sources/LumaApp/Launcher/NormalizedQueryState.swift")
    #expect(FileManager.default.fileExists(atPath: normalizedPath.path) == false)
}

@Test func launcherRootControllerUsesQueryViewSnapshot() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let path = root.appending(path: "Sources/LumaApp/Launcher/LauncherRootController.swift")
    let source = try String(contentsOf: path, encoding: .utf8)
    #expect(source.contains("QueryView(raw:"))
    #expect(source.contains("lastQueryView"))
}
