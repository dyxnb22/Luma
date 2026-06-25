import Testing
@testable import LumaServices

@Suite struct WindowFocusMatcherTests {
    private let cursorBundle = "com.todesktop.230313mzl4w4u92"
    private let appName = "Cursor"

    private func titled(_ pairs: [(Int, String)]) -> [WindowFocusMatcher.TitledWindow] {
        pairs.map { ($0.0, $0.1) }
    }

    @Test func matchesCursorProjectBySidebarLabel() {
        let windows = titled([
            (1, "Luma — Cursor"),
            (2, "big-market-ai-platform — Cursor"),
            (3, "LeetCode — Cursor")
        ])
        let index = WindowFocusMatcher.matchingIndex(
            in: windows,
            queryTitle: "big-market-ai-platform",
            bundleID: cursorBundle,
            appName: appName
        )
        #expect(index == 2)
    }

    @Test func matchesCursorProjectFromRawWindowTitle() {
        let windows = titled([
            (4, "README.md — Luma — Cursor"),
            (5, "App.swift — big-market-ai-platform — Cursor"),
            (6, "Solution.java — LeetCode — Cursor")
        ])
        let index = WindowFocusMatcher.matchingIndex(
            in: windows,
            queryTitle: "big-market-ai-platform",
            bundleID: cursorBundle,
            appName: appName
        )
        #expect(index == 5)
    }

    @Test func preservesAXWindowIndexWhenUntitledWindowsExist() {
        let windows = titled([
            (1, "Luma — Cursor"),
            (3, "big-market-ai-platform — Cursor"),
            (5, "LeetCode — Cursor")
        ])
        let index = WindowFocusMatcher.matchingIndex(
            in: windows,
            queryTitle: "LeetCode",
            bundleID: cursorBundle,
            appName: appName
        )
        #expect(index == 5)
    }

    @Test func doesNotMatchAllCursorWindowsToCursorSuffix() {
        let windows = titled([
            (0, "Luma — Cursor"),
            (1, "big-market-ai-platform — Cursor"),
            (2, "LeetCode — Cursor")
        ])
        let index = WindowFocusMatcher.matchingIndex(
            in: windows,
            queryTitle: "LeetCode — Cursor",
            bundleID: cursorBundle,
            appName: appName
        )
        #expect(index == 2)
    }

    @Test func returnsNilWhenMultipleWindowsAndNoMatch() {
        let windows = titled([
            (0, "Luma — Cursor"),
            (1, "big-market-ai-platform — Cursor")
        ])
        let index = WindowFocusMatcher.matchingIndex(
            in: windows,
            queryTitle: "unknown-project",
            bundleID: cursorBundle,
            appName: appName
        )
        #expect(index == nil)
    }
}
