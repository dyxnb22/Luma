import Foundation
import LumaCore
import LumaInfrastructure

/// Coalesces UI snapshot applies to at most one per 16 ms frame (matches query dispatcher cadence).
/// The first apply in a session flushes immediately; subsequent applies in the same burst wait up to 16 ms.
@MainActor
final class LauncherSnapshotApplyCoalescer {
    static let interval: Duration = .milliseconds(16)

    private var pending: ResultSnapshot?
    private var task: Task<Void, Never>?
    private var lastApplyTime: ContinuousClock.Instant?
    private let onApply: (ResultSnapshot) -> Void

    init(onApply: @escaping (ResultSnapshot) -> Void) {
        self.onApply = onApply
    }

    func enqueue(_ snapshot: ResultSnapshot) {
        if pending != nil || task != nil {
            LauncherPerfCounters.increment(.snapshotApplyCoalesced)
        }
        pending = snapshot
        task?.cancel()
        task = Task { [weak self] in
            guard let self else { return }
            let now = ContinuousClock.now
            if let last = self.lastApplyTime {
                let elapsed = now - last
                if elapsed < Self.interval {
                    try? await Task.sleep(for: Self.interval - elapsed)
                }
            }
            guard !Task.isCancelled else { return }
            self.flush()
        }
    }

    func flushNow() {
        task?.cancel()
        task = nil
        flush()
    }

    func cancel() {
        task?.cancel()
        task = nil
        pending = nil
    }

    private func flush() {
        guard let snapshot = pending else { return }
        pending = nil
        lastApplyTime = .now
        onApply(snapshot)
    }
}
