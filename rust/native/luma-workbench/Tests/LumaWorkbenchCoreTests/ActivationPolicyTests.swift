import XCTest

@testable import LumaWorkbenchCore

final class ActivationPolicyTests: XCTestCase {
    private let ownPID: pid_t = 501
    private let editor = PreviousApplication(processIdentifier: 900, bundleIdentifier: "com.apple.dt.Xcode")

    func testHotKeyWhileHiddenShowsAndRemembersFrontmostApplication() {
        var policy = ActivationPolicy()

        let action = policy.toggle(
            applicationIsActive: false,
            frontmostApplication: editor,
            ownProcessIdentifier: ownPID,
            sessionIsRunning: true
        )

        XCTAssertEqual(action, .show(startSession: false))
        XCTAssertTrue(policy.isVisible)
        XCTAssertEqual(policy.previousApplication, editor)
    }

    func testHotKeyWhileVisibleAndActiveHidesAndHandsFocusBack() {
        var policy = ActivationPolicy()
        _ = policy.toggle(
            applicationIsActive: false,
            frontmostApplication: editor,
            ownProcessIdentifier: ownPID,
            sessionIsRunning: true
        )

        let action = policy.toggle(
            applicationIsActive: true,
            frontmostApplication: PreviousApplication(processIdentifier: ownPID, bundleIdentifier: "com.luma.next.workbench"),
            ownProcessIdentifier: ownPID,
            sessionIsRunning: true
        )

        XCTAssertEqual(action, .hide(reactivate: editor))
        XCTAssertFalse(policy.isVisible)
        XCTAssertNil(policy.previousApplication, "the remembered app is consumed by hiding")
    }

    func testHotKeyWhileVisibleAfterSessionEndsRestartsImmediately() {
        var policy = ActivationPolicy(isVisible: true)

        let action = policy.toggle(
            applicationIsActive: true,
            frontmostApplication: nil,
            ownProcessIdentifier: ownPID,
            sessionIsRunning: false
        )

        XCTAssertEqual(action, .show(startSession: true))
        XCTAssertTrue(policy.isVisible)
    }

    /// Visible but buried behind another app: the hotkey must bring Luma forward, not hide it.
    func testHotKeyWhileVisibleButInactiveShows() {
        var policy = ActivationPolicy(isVisible: true)

        let action = policy.toggle(
            applicationIsActive: false,
            frontmostApplication: editor,
            ownProcessIdentifier: ownPID,
            sessionIsRunning: true
        )

        XCTAssertEqual(action, .show(startSession: false))
        XCTAssertEqual(policy.previousApplication, editor)
    }

    func testLumaIsNeverRememberedAsThePreviousApplication() {
        var policy = ActivationPolicy()
        let luma = PreviousApplication(processIdentifier: ownPID, bundleIdentifier: "com.luma.next.workbench")

        _ = policy.toggle(
            applicationIsActive: false,
            frontmostApplication: luma,
            ownProcessIdentifier: ownPID,
            sessionIsRunning: true
        )

        XCTAssertNil(policy.previousApplication)
        XCTAssertEqual(policy.hide(), .hide(reactivate: nil))
    }

    func testShowStartsASessionOnlyWhenNoneIsRunning() {
        var policy = ActivationPolicy()

        XCTAssertEqual(
            policy.show(frontmostApplication: nil, ownProcessIdentifier: ownPID, sessionIsRunning: false),
            .show(startSession: true)
        )
        XCTAssertEqual(
            policy.show(frontmostApplication: nil, ownProcessIdentifier: ownPID, sessionIsRunning: true),
            .show(startSession: false)
        )
    }

    /// Rapid toggling must land on a well-defined state rather than accumulating shows.
    func testRepeatedTogglingAlternatesWithoutAccumulatingState() {
        var policy = ActivationPolicy()
        var visible = false

        for _ in 0..<20 {
            let action = policy.toggle(
                applicationIsActive: visible,
                frontmostApplication: editor,
                ownProcessIdentifier: ownPID,
                sessionIsRunning: true
            )
            switch action {
            case .show: visible = true
            case .hide: visible = false
            }
            XCTAssertEqual(policy.isVisible, visible)
        }

        XCTAssertFalse(visible)
        XCTAssertNil(policy.previousApplication)
    }

    func testPreviousApplicationIsReactivatedOnlyWhenItIsStillTheSameApplication() {
        let recycled = PreviousApplication(processIdentifier: 900, bundleIdentifier: "com.apple.Terminal")

        XCTAssertTrue(PreviousApplicationTracking.shouldReactivate(remembered: editor, current: editor))
        XCTAssertFalse(
            PreviousApplicationTracking.shouldReactivate(remembered: editor, current: recycled),
            "a recycled PID must not steal focus for a different app"
        )
    }
}

final class HotKeyDebouncerTests: XCTestCase {
    func testAcceptsTheFirstEvent() {
        var debouncer = HotKeyDebouncer(minimumInterval: 0.12)
        XCTAssertTrue(debouncer.shouldAccept(at: 100))
    }

    func testDropsEventsInsideTheInterval() {
        var debouncer = HotKeyDebouncer(minimumInterval: 0.12)
        XCTAssertTrue(debouncer.shouldAccept(at: 100))
        XCTAssertFalse(debouncer.shouldAccept(at: 100.05))
        XCTAssertFalse(debouncer.shouldAccept(at: 100.11))
    }

    func testAcceptsAgainAfterTheInterval() {
        var debouncer = HotKeyDebouncer(minimumInterval: 0.12)
        XCTAssertTrue(debouncer.shouldAccept(at: 100))
        XCTAssertFalse(debouncer.shouldAccept(at: 100.05))
        XCTAssertTrue(debouncer.shouldAccept(at: 100.2))
    }

    /// A burst of key repeats must collapse to a single activation.
    func testBurstCollapsesToOneActivation() {
        var debouncer = HotKeyDebouncer(minimumInterval: 0.12)
        let accepted = stride(from: 0.0, to: 0.1, by: 0.01).filter { debouncer.shouldAccept(at: $0) }
        XCTAssertEqual(accepted.count, 1)
    }

    func testHotKeyDefinitionIsOptionSpace() {
        XCTAssertEqual(HotKeyDefinition.optionSpace.keyCode, 0x31, "kVK_Space")
        XCTAssertEqual(HotKeyDefinition.optionSpace.carbonModifiers, 2048, "Carbon optionKey")
    }
}
