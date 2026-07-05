import Foundation
import LumaCore
import AppKit
import Testing
@testable import LumaServices

private let testBrowserBundleID = "com.test.SlowBrowser"

private struct FailingBrowserAdapter: BrowserAdapter {
    let bundleID: String
    let applicationName: String
    let error: Error

    func fetchTabs(runner: AppleScriptRunner) async throws -> [TabRecord] {
        throw error
    }

    func activate(record: TabRecord, runner: AppleScriptRunner) async throws {}
}

private struct SlowBrowserAdapter: BrowserAdapter {
    let bundleID: String
    let applicationName: String
    let delay: Duration

    func fetchTabs(runner: AppleScriptRunner) async throws -> [TabRecord] {
        try await Task.sleep(for: delay)
        return [
            TabRecord(
                bundleID: bundleID,
                browserName: applicationName,
                windowIndex: 1,
                tabIndex: 1,
                title: "Slow Tab",
                url: "https://example.com"
            )
        ]
    }

    func activate(record: TabRecord, runner: AppleScriptRunner) async throws {}
}

private func makeTestService(
    adapters: [any BrowserAdapter],
    runningBundleIDs: Set<String> = [testBrowserBundleID]
) -> BrowserTabsService {
    BrowserTabsService(
        adapters: adapters,
        runningBundleIDs: { runningBundleIDs }
    )
}

@Test func browserTabsServiceEmptyCacheReturnsImmediately() async throws {
    let service = makeTestService(
        adapters: [
            SlowBrowserAdapter(
                bundleID: testBrowserBundleID,
                applicationName: "Slow Browser",
                delay: .seconds(2)
            )
        ]
    )

    let clock = ContinuousClock()
    let start = clock.now
    let tabs = await service.searchableTabs()
    let elapsed = start.duration(to: clock.now)

    #expect(tabs.isEmpty)
    #expect(elapsed < .milliseconds(500))

    var refreshed: [TabRecord] = []
    let deadline = ContinuousClock.now + .seconds(5)
    while ContinuousClock.now < deadline {
        refreshed = await service.searchableTabs()
        if refreshed.count == 1 { break }
        try await Task.sleep(for: .milliseconds(100))
    }
    #expect(refreshed.count == 1)
    #expect(refreshed[0].title == "Slow Tab")
}

@Test func browserTabsServiceSurfacesAutomationDeniedDiagnostic() async {
    let service = makeTestService(
        adapters: [
            FailingBrowserAdapter(
                bundleID: testBrowserBundleID,
                applicationName: "Test Browser",
                error: AppleScriptRunner.RunnerError.failed("Not authorized to send Apple events to Safari. (-1743)")
            )
        ]
    )
    await service.refresh()
    let diagnostic = await service.lastDiagnostic()
    #expect(diagnostic?.kind == .permissionRequired)
    #expect(diagnostic?.message.localizedCaseInsensitiveContains("automation denied") == true)
}

@Test func browserTabsServiceSurfacesTimeoutDiagnostic() async {
    let service = makeTestService(
        adapters: [
            FailingBrowserAdapter(
                bundleID: testBrowserBundleID,
                applicationName: "Test Browser",
                error: AppleScriptRunner.RunnerError.timedOut
            )
        ]
    )
    await service.refresh()
    let diagnostic = await service.lastDiagnostic()
    #expect(diagnostic?.kind == .error)
    #expect(diagnostic?.message.localizedCaseInsensitiveContains("timed out") == true)
}
