import Foundation

public enum Timeout {
    /// Returns the first finished value from `operation` or `nil` when `duration` elapses.
    /// Returns when `duration` elapses even if `operation` keeps running; the detached work task is
    /// cancelled but non-cooperative module warmup may still mutate actor state after the caller continues.
    public static func run<T: Sendable>(
        after duration: Duration,
        operation: @Sendable @escaping () async -> T
    ) async -> T? {
        await withCheckedContinuation { (continuation: CheckedContinuation<T?, Never>) in
            let gate = RaceGate(continuation)
            let work = Task.detached(priority: .high) {
                let value = await operation()
                gate.finish(value)
            }
            let seconds = max(Self.seconds(from: duration), 0.001)
            DispatchQueue.global(qos: .utility).asyncAfter(deadline: .now() + seconds) {
                gate.finish(nil)
                work.cancel()
            }
        }
    }

    private static func seconds(from duration: Duration) -> TimeInterval {
        let components = duration.components
        return Double(components.seconds) + Double(components.attoseconds) / 1_000_000_000_000_000_000
    }
}

private final class RaceGate<T: Sendable>: @unchecked Sendable {
    private let lock = NSLock()
    private var finished = false
    private let continuation: CheckedContinuation<T?, Never>

    init(_ continuation: CheckedContinuation<T?, Never>) {
        self.continuation = continuation
    }

    func finish(_ value: sending T?) {
        lock.lock()
        defer { lock.unlock() }
        guard !finished else { return }
        finished = true
        continuation.resume(returning: value)
    }
}
