import Foundation
import Testing
@testable import LumaServices

private final class MockAXWindowEnumerator: AXWindowEnumerating, @unchecked Sendable {
    private let lock = NSLock()
    private var _copyOnScreenCallCount = 0
    let granted: Bool
    let windowsByPID: [Int32: [CGWindowBoundsInfo]]
    let axWindowsByPID: [Int32: [OpenWindowSnapshot]]

    var copyOnScreenCallCount: Int {
        lock.lock()
        defer { lock.unlock() }
        return _copyOnScreenCallCount
    }

    init(
        granted: Bool = true,
        windowsByPID: [Int32: [CGWindowBoundsInfo]] = [:],
        axWindowsByPID: [Int32: [OpenWindowSnapshot]] = [:]
    ) {
        self.granted = granted
        self.windowsByPID = windowsByPID
        self.axWindowsByPID = axWindowsByPID
    }

    var isAccessibilityGranted: Bool { granted }

    func copyOnScreenWindowsByPID() -> [Int32: [CGWindowBoundsInfo]] {
        lock.lock()
        _copyOnScreenCallCount += 1
        lock.unlock()
        return windowsByPID
    }

    func enumerateWindows(for pid: Int32, appName: String, cgWindows: [CGWindowBoundsInfo]) -> [OpenWindowSnapshot] {
        axWindowsByPID[pid] ?? []
    }
}

@Suite struct RunningAppsWindowCollectorTests {
    @Test func copiesOnScreenWindowsOnceForAllApps() {
        let apps = [
            RunningAppMetadata(pid: 10, bundleID: "a.one", name: "One", appURLPath: "/Apps/One.app"),
            RunningAppMetadata(pid: 20, bundleID: "a.two", name: "Two", appURLPath: "/Apps/Two.app")
        ]
        let cgWindows: [Int32: [CGWindowBoundsInfo]] = [
            10: [CGWindowBoundsInfo(windowID: 1, pid: 10, title: "One", bounds: .zero)],
            20: [CGWindowBoundsInfo(windowID: 2, pid: 20, title: "Two", bounds: .zero)]
        ]
        let axWindows: [Int32: [OpenWindowSnapshot]] = [
            10: [OpenWindowSnapshot(windowID: 1, pid: 10, title: "One", isMain: true, isMinimized: false)],
            20: [OpenWindowSnapshot(windowID: 2, pid: 20, title: "Two", isMain: true, isMinimized: false)]
        ]
        let enumerator = MockAXWindowEnumerator(
            windowsByPID: cgWindows,
            axWindowsByPID: axWindows
        )

        let result = RunningAppsWindowCollector.windowsByPID(for: apps, using: enumerator)

        #expect(enumerator.copyOnScreenCallCount == 1)
        #expect(result[10]?.count == 1)
        #expect(result[20]?.count == 1)
    }

    @Test func skipsWindowEnumerationWhenAccessibilityDenied() {
        let apps = [
            RunningAppMetadata(pid: 10, bundleID: "a.one", name: "One", appURLPath: "/Apps/One.app")
        ]
        let enumerator = MockAXWindowEnumerator(granted: false)

        let result = RunningAppsWindowCollector.windowsByPID(for: apps, using: enumerator)

        #expect(enumerator.copyOnScreenCallCount == 0)
        #expect(result.isEmpty)
    }
}
