import Foundation
import LumaCore
import AppKit
import Testing
@testable import LumaServices

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

@Test func browserTabsServiceEmptyCacheReturnsImmediately() async throws {
    guard let runningBundleID = await MainActor.run(body: {
        NSWorkspace.shared.runningApplications.compactMap(\.bundleIdentifier).first
    }) else { return }

    let service = BrowserTabsService(
        adapters: [
            SlowBrowserAdapter(
                bundleID: runningBundleID,
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

    try await Task.sleep(for: .seconds(3))
    let refreshed = await service.searchableTabs()
    #expect(refreshed.count == 1)
    #expect(refreshed[0].title == "Slow Tab")
}

@Test func browserTabsServiceSurfacesAutomationDeniedDiagnostic() async {
    guard let runningBundleID = await MainActor.run(body: {
        NSWorkspace.shared.runningApplications.compactMap(\.bundleIdentifier).first
    }) else { return }

    let service = BrowserTabsService(
        adapters: [
            FailingBrowserAdapter(
                bundleID: runningBundleID,
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
    guard let runningBundleID = await MainActor.run(body: {
        NSWorkspace.shared.runningApplications.compactMap(\.bundleIdentifier).first
    }) else { return }

    let service = BrowserTabsService(
        adapters: [
            FailingBrowserAdapter(
                bundleID: runningBundleID,
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
