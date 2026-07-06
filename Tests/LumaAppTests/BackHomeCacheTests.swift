import Foundation
import LumaCore
import LumaInfrastructure
import LumaServices
import Testing
@testable import LumaApp

@Test func homeCoordinatorReusesCachedSnapshotWithoutExtraBuild() async {
    LauncherPerfCounters.reset()
    let trackerURL = FileManager.default.temporaryDirectory
        .appendingPathComponent("LumaAppTests-\(UUID().uuidString)-activation.json")
    let tracker = AppActivationTracker(url: trackerURL)
    let openApps = OpenAppsHomeProvider(
        appActivationTracker: tracker,
        windowEnumerator: NoopAXWindowEnumerator()
    )
    let coordinator = LauncherHomeCoordinator(openApps: openApps)

    _ = await coordinator.snapshot()
    let firstGeneration = await coordinator.currentSnapshotGeneration()
    let firstHomeSnapshotCount = LauncherPerfCounters.count(for: .homeSnapshot)

    _ = await coordinator.snapshot()
    let secondGeneration = await coordinator.currentSnapshotGeneration()
    let secondHomeSnapshotCount = LauncherPerfCounters.count(for: .homeSnapshot)

    #expect(firstGeneration == secondGeneration)
    #expect(secondHomeSnapshotCount == firstHomeSnapshotCount)
}

private struct NoopAXWindowEnumerator: AXWindowEnumerating {
    var isAccessibilityGranted: Bool { false }

    func copyOnScreenWindowsByPID() -> [Int32: [CGWindowBoundsInfo]] { [:] }

    func enumerateWindows(for pid: Int32, appName: String, cgWindows: [CGWindowBoundsInfo]) -> [OpenWindowSnapshot] {
        _ = pid
        _ = appName
        _ = cgWindows
        return []
    }
}
