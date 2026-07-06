import Foundation
import LumaCore
import Testing

@Test func searchDetailModeBeginEndRestoresSuspendedQuery() {
    var state = LauncherSearchDetailModeState(visibleQuery: "clip foo", isEditable: true)
    state = LauncherSearchDetailMode.beginDetailMode(state, moduleTitle: "Clipboard")
    let (ended, restored) = LauncherSearchDetailMode.endDetailMode(state)
    #expect(ended.isEditable)
    #expect(ended.suspendedQuery == nil)
    #expect(restored == "clip foo")
}

@Test func searchDetailModeBeginCancelDropsSuspendedQuery() {
    var state = LauncherSearchDetailModeState(visibleQuery: "word", isEditable: true)
    state = LauncherSearchDetailMode.beginDetailMode(state, moduleTitle: "Wordbook")
    state = LauncherSearchDetailMode.cancelDetailMode(state)
    #expect(state.suspendedQuery == nil)
    #expect(state.isEditable)
}

@Test func searchDetailModeClearStuckRestoresEditability() {
    let stuck = LauncherSearchDetailModeState(isEditable: false)
    let fixed = LauncherSearchDetailMode.clearStuckDetailModeState(stuck)
    #expect(fixed.isEditable)
    #expect(fixed.suspendedQuery == nil)
}

@Test func lumaSearchBarWiresLauncherSearchDetailMode() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let path = root.appending(path: "Sources/LumaApp/Launcher/LumaSearchBar.swift")
    let source = try String(contentsOf: path, encoding: .utf8)
    #expect(source.contains("LauncherSearchDetailModeState"))
    #expect(source.contains("LauncherSearchDetailMode.beginDetailMode"))
    #expect(source.contains("LauncherSearchDetailMode.endDetailMode"))
    #expect(source.contains("LauncherSearchDetailMode.cancelDetailMode"))
}
