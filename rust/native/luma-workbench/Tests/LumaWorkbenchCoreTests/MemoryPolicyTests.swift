import XCTest

@testable import LumaWorkbenchCore

final class MemoryPolicyTests: XCTestCase {
    func testTrackerRecordsLatestAndPeakAcrossBothProcesses() {
        var tracker = ResidentMemoryTracker()
        tracker.record(.init(hostBytes: 20, childBytes: 30))
        tracker.record(.init(hostBytes: 10, childBytes: 15))

        XCTAssertEqual(tracker.latest, .init(hostBytes: 10, childBytes: 15))
        XCTAssertEqual(tracker.peakTotalBytes, 50)
    }

    func testTotalBytesSaturatesInsteadOfOverflowing() {
        let snapshot = ResidentMemorySnapshot(hostBytes: .max, childBytes: 1)
        XCTAssertEqual(snapshot.totalBytes, .max)
    }

    func testCriticalPressureKeepsLessScrollbackThanWarning() {
        XCTAssertLessThan(
            MemoryPressurePolicy.scrollbackLines(for: .critical),
            MemoryPressurePolicy.scrollbackLines(for: .warning)
        )
    }
}
