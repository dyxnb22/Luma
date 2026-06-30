import Foundation
import Testing
import LumaCore

@Test func timeoutReturnsBeforeSlowOperationFinishes() async {
    let start = ContinuousClock().now
    let result = await Timeout.run(after: .milliseconds(50)) {
        try? await Task.sleep(for: .seconds(2))
        return 42
    }
    let elapsed = start.duration(to: ContinuousClock().now)

    #expect(result == nil)
    #expect(elapsed < .seconds(2))
}

@Test func timeoutReturnsOperationValueWhenFast() async {
    let result = await Timeout.run(after: .milliseconds(200)) {
        7
    }
    #expect(result == 7)
}

@Test func timeoutIgnoresLateOperationResultAfterDeadline() async {
    let gate = AsyncGate()
    let result = await Timeout.run(after: .milliseconds(50)) {
        await gate.wait()
        return "late"
    }
    #expect(result == nil)
    await gate.open()
}

private actor AsyncGate {
    private var continuation: CheckedContinuation<Void, Never>?

    func wait() async {
        await withCheckedContinuation { continuation = $0 }
    }

    func open() {
        continuation?.resume()
        continuation = nil
    }
}
