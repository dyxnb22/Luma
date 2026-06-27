import AppKit
import Testing
@testable import LumaModules
import LumaServices

@Test func browserTabParserParsesTSVRows() {
    let rows = BrowserTabParser.parseTSV("1\t2\tGitHub\thttps://github.com\n", bundleID: "com.apple.Safari", browserName: "Safari")
    #expect(rows.count == 1)
    #expect(rows[0].windowIndex == 1)
    #expect(rows[0].tabIndex == 2)
}

@Test func browserTabParserSkipsMalformedRows() {
    let rows = BrowserTabParser.parseTSV("bad\t2\tNope\thttps://example.com\n1\t1\tOK\thttps://ok.test\n", bundleID: "b", browserName: "B")
    #expect(rows.count == 1)
    #expect(rows[0].title == "OK")
}

@Test func browserTabsIndexSearchesTitleAndURL() {
    let tabs = [
        TabRecord(bundleID: "b", browserName: "B", windowIndex: 1, tabIndex: 1, title: "Docs", url: "https://developer.apple.com"),
        TabRecord(bundleID: "b", browserName: "B", windowIndex: 1, tabIndex: 2, title: "GitHub", url: "https://github.com/openai")
    ]
    #expect(BrowserTabsIndex.search(tabs, query: "github").first?.tabIndex == 2)
    #expect(BrowserTabsIndex.search(tabs, query: "developer").first?.tabIndex == 1)
    #expect(BrowserTabsIndex.search(tabs, query: "zzzz").isEmpty)
}

@Test func browserTabsIndexPreservesDistinctTabsWithSameURL() {
    let tabs = [
        TabRecord(bundleID: "com.apple.Safari", browserName: "Safari", windowIndex: 1, tabIndex: 1, title: "GitHub A", url: "https://github.com"),
        TabRecord(bundleID: "com.apple.Safari", browserName: "Safari", windowIndex: 2, tabIndex: 1, title: "GitHub B", url: "https://github.com"),
        TabRecord(bundleID: "com.google.Chrome", browserName: "Chrome", windowIndex: 1, tabIndex: 1, title: "GitHub C", url: "https://github.com")
    ]
    let matches = BrowserTabsIndex.search(tabs, query: "github")
    #expect(matches.count == 3)
    #expect(Set(matches.map { "\($0.bundleID)|\($0.windowIndex)|\($0.tabIndex)" }).count == 3)
}

@Test func browserTabsIndexDedupesExactDuplicateRecords() {
    let original = TabRecord(
        bundleID: "com.apple.Safari",
        browserName: "Safari",
        windowIndex: 1,
        tabIndex: 1,
        title: "GitHub",
        url: "https://github.com"
    )
    let matches = BrowserTabsIndex.search([original, original], query: "github")
    #expect(matches.count == 1)
}

@Test(.tags(.integration), .enabled(if: IntegrationTestSettings.enabled))
func appleScriptRunnerFetchesSafariTabTSV() async throws {
    let running = await MainActor.run {
        NSWorkspace.shared.runningApplications.contains { $0.bundleIdentifier == "com.apple.Safari" }
    }
    guard running else { return }

    let script = """
    tell application "Safari"
      set delim to ASCII character 9
      set out to ""
      repeat with w from 1 to count of windows
        set tabsOfW to tabs of window w
        repeat with t from 1 to count of tabsOfW
          set theTab to item t of tabsOfW
          set out to out & w & delim & t & delim & (name of theTab) & delim & (URL of theTab) & linefeed
        end repeat
      end repeat
      return out
    end tell
    """
    let runner = AppleScriptRunner()
    let output = try await runner.run(script, timeout: 2.0)
    guard !output.isEmpty else { return }
    let rows = BrowserTabParser.parseTSV(output, bundleID: "com.apple.Safari", browserName: "Safari")
    guard !rows.isEmpty else { return }
    #expect(rows[0].url.hasPrefix("http") || rows[0].url.hasPrefix("favorites:"))
}

@Test(.tags(.integration), .enabled(if: IntegrationTestSettings.enabled))
func browserTabsServiceFetchesSafariTabsWhenRunning() async {
    let running = await MainActor.run {
        NSWorkspace.shared.runningApplications.contains { $0.bundleIdentifier == "com.apple.Safari" }
    }
    guard running else { return }

    let service = BrowserTabsService()
    await service.refresh()
    let tabs = await service.searchableTabs()
    guard !tabs.isEmpty else { return }
    #expect(BrowserTabsIndex.search(tabs, query: "http").isEmpty == false)
}
