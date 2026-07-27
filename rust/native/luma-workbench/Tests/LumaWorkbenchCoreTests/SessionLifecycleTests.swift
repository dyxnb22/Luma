import XCTest

@testable import LumaWorkbenchCore

final class SessionLifecycleTests: XCTestCase {
    func testFirstShowStartsASession() {
        let lifecycle = SessionLifecycle()
        XCTAssertEqual(lifecycle.state, .notStarted)
        XCTAssertTrue(lifecycle.needsStartBeforeShowing)
    }

    func testRunningSessionIsReusedAcrossHideAndShow() {
        var lifecycle = SessionLifecycle()
        lifecycle.markStarted()

        XCTAssertTrue(lifecycle.isRunning)
        XCTAssertFalse(lifecycle.needsStartBeforeShowing, "hiding must not restart the TUI")
        XCTAssertNil(lifecycle.terminationNotice)
    }

    func testExitedSessionRestartsOnTheNextShow() {
        var lifecycle = SessionLifecycle()
        lifecycle.markStarted()
        lifecycle.markTerminated(swiftTermWaitStatus: 0)

        XCTAssertEqual(lifecycle.state, .exited(.exited(code: 0)))
        XCTAssertFalse(lifecycle.isRunning)
        XCTAssertTrue(lifecycle.needsStartBeforeShowing)
    }

    func testRestartLeavesTheLifecycleRunningAgain() {
        var lifecycle = SessionLifecycle()
        lifecycle.markStarted()
        lifecycle.markTerminated(swiftTermWaitStatus: 256)
        lifecycle.markStarted()

        XCTAssertTrue(lifecycle.isRunning)
        XCTAssertFalse(lifecycle.needsStartBeforeShowing)
    }

    func testTerminationNoticeReportsTheExitCode() {
        var lifecycle = SessionLifecycle()
        lifecycle.markStarted()
        lifecycle.markTerminated(swiftTermWaitStatus: 130 << 8)

        let notice = try? XCTUnwrap(lifecycle.terminationNotice)
        XCTAssertEqual(
            notice,
            "[luma tui ended — exit code 130. Press ⌘Space to start a new session.]"
        )
    }

    func testTerminationNoticeStaysHonestWithoutAnExitCode() {
        var lifecycle = SessionLifecycle()
        lifecycle.markStarted()
        lifecycle.markTerminated(swiftTermWaitStatus: nil)

        XCTAssertEqual(lifecycle.terminationNotice?.contains("unknown exit status"), true)
    }

    func testSwiftTermRawWaitStatusDecodesNormalExit() {
        XCTAssertEqual(
            SessionTermination.fromSwiftTermWaitStatus(7 << 8),
            .exited(code: 7)
        )
    }

    func testSwiftTermRawWaitStatusDecodesSignal() {
        XCTAssertEqual(
            SessionTermination.fromSwiftTermWaitStatus(15),
            .signaled(signal: 15)
        )
    }

    func testSignalNoticeIsNotMisreportedAsExitCode() {
        var lifecycle = SessionLifecycle()
        lifecycle.markStarted()
        lifecycle.markTerminated(swiftTermWaitStatus: 15)

        XCTAssertEqual(
            lifecycle.terminationNotice,
            "[luma tui ended — signal 15. Press ⌘Space to start a new session.]"
        )
    }
}

final class ChildProcessGroupTests: XCTestCase {
    func testPositiveChildPIDMapsToNegativeProcessGroupTarget() {
        XCTAssertEqual(ChildProcessGroup.signalTarget(forLeader: 4321), -4321)
    }

    func testUnsafePIDsAreRejected() {
        XCTAssertNil(ChildProcessGroup.signalTarget(forLeader: 1))
        XCTAssertNil(ChildProcessGroup.signalTarget(forLeader: 0))
        XCTAssertNil(ChildProcessGroup.signalTarget(forLeader: -1))
    }
}

final class TerminalGeometryTests: XCTestCase {
    func testContentSizeSnapsDownToWholeCells() {
        let size = TerminalGeometry.integralContentSize(
            preferred: CGSize(width: 960, height: 700),
            cell: CGSize(width: 7, height: 15)
        )

        XCTAssertEqual(size.width.truncatingRemainder(dividingBy: 7), 0, accuracy: 0.0001)
        XCTAssertEqual(size.height.truncatingRemainder(dividingBy: 15), 0, accuracy: 0.0001)
        XCTAssertLessThanOrEqual(size.width, 960)
        XCTAssertLessThanOrEqual(size.height, 700)
    }

    func testReservedScrollerWidthIsExcludedFromTheGrid() {
        let reserved: CGFloat = 15
        let size = TerminalGeometry.integralContentSize(
            preferred: CGSize(width: 960, height: 700),
            cell: CGSize(width: 7, height: 15),
            reservedWidth: reserved
        )

        XCTAssertEqual((size.width - reserved).truncatingRemainder(dividingBy: 7), 0, accuracy: 0.0001)
    }

    func testTinyPreferredSizeStillYieldsAUsableGrid() {
        let size = TerminalGeometry.integralContentSize(
            preferred: CGSize(width: 10, height: 10),
            cell: CGSize(width: 7, height: 15)
        )

        XCTAssertEqual(size.width, CGFloat(TerminalGeometry.minimumColumns) * 7)
        XCTAssertEqual(size.height, CGFloat(TerminalGeometry.minimumRows) * 15)
    }

    func testDegenerateCellSizeFallsBackToThePreferredSize() {
        let preferred = CGSize(width: 960, height: 700)
        XCTAssertEqual(
            TerminalGeometry.integralContentSize(preferred: preferred, cell: .zero),
            preferred
        )
    }
}
