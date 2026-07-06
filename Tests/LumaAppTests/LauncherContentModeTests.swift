import Foundation
import LumaCore
import Testing

@Test func launcherContentModeDerivedFlags() {
    #expect(LauncherContentMode.home.showingDetail == false)
    #expect(LauncherContentMode.home.showingResults == false)
    #expect(LauncherContentMode.results.showingResults)
    #expect(LauncherContentMode.detail(.notes).showingDetail)
    #expect(LauncherContentMode.detail(.notes).detailModuleID == .notes)
}

@Test func impossibleDetailAndResultsNotRepresentableInSingleMode() {
    let modes: [LauncherContentMode] = [.home, .results, .detail(.clipboard)]
    for mode in modes {
        if mode.showingDetail {
            #expect(mode.showingResults == false)
        }
    }
}

@Test func contentCoordinatorUsesLauncherContentMode() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let path = root.appending(path: "Sources/LumaApp/Launcher/LauncherContentCoordinator.swift")
    let source = try String(contentsOf: path, encoding: .utf8)
    #expect(source.contains("LauncherContentMode"))
    #expect(source.contains("var mode: LauncherContentMode"))
}
