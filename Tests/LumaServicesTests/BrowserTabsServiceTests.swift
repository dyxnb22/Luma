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
