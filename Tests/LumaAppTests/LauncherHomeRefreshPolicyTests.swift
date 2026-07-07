import Foundation
import Testing
@testable import LumaApp

@Test @MainActor func abandonedHotkeyMarkDoesNotRecordHomeLatency() {
    HomeLatencyTracker.markHotkey()
    HomeLatencyTracker.abandonPendingHotkeyMark()
    #expect(HomeLatencyTracker.markHomeRendered() == nil)
}

// Wiring-only: ensures window controller routes background refresh through the cache-warm intent.
// Behavioral policy is covered by `LauncherHomeRefreshRepaintPolicyTests` in LumaCoreTests.
@Test func backgroundHomeRefreshUsesVisibilitySessionAndCacheIntent() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let windowPath = root.appending(path: "Sources/LumaApp/Launcher/LauncherWindowController.swift")
    let windowSource = try String(contentsOf: windowPath, encoding: .utf8)
    #expect(windowSource.contains("guard !visibilitySession.isVisible else { return }"))
    #expect(windowSource.contains("refreshHome(intent: .backgroundCacheWarm)"))
}
