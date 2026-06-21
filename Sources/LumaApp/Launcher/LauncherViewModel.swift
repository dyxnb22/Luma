import Foundation
import LumaCore

@MainActor
final class LauncherViewModel {
    private var sequence: UInt64 = 0
    private var task: Task<Void, Never>?
    private let dispatcher: QueryDispatcher
    private var issuedAtBySequence: [UInt64: ContinuousClock.Instant] = [:]
    private var latencySamples: [Double] = []
    var onSnapshot: (@MainActor (ResultSnapshot) -> Void)?

    init(dispatcher: QueryDispatcher) {
        self.dispatcher = dispatcher
    }

    func queryChanged(_ text: String, issuedAt: ContinuousClock.Instant) {
        task?.cancel()
        sequence &+= 1
        issuedAtBySequence[sequence] = issuedAt
        let query = Query(raw: text, sequence: sequence)
        task = Task {
            try? await Task.sleep(for: .milliseconds(12))
            guard !Task.isCancelled else { return }
            await dispatcher.dispatch(query) { [weak self] snapshot in
                await MainActor.run {
                    guard let self, snapshot.querySequence == self.sequence else { return }
                    self.onSnapshot?(snapshot)
                }
            }
        }
    }

    func p95LatencyMilliseconds(for sequence: UInt64) -> Double? {
        guard let issuedAt = issuedAtBySequence.removeValue(forKey: sequence) else { return nil }
        let elapsed = issuedAt.duration(to: .now)
        let ms = Double(elapsed.components.seconds) * 1000
            + Double(elapsed.components.attoseconds) / 1_000_000_000_000_000
        latencySamples.append(ms)
        if latencySamples.count > 100 {
            latencySamples.removeFirst(latencySamples.count - 100)
        }
        return p95(of: latencySamples)
    }

    func cancel() {
        task?.cancel()
        task = nil
        issuedAtBySequence.removeAll()
    }

    func recentFrecency(limit: Int = 8) async -> [ResultItem] {
        await dispatcher.recentFrecency(limit: limit)
    }

    private func p95(of samples: [Double]) -> Double {
        guard !samples.isEmpty else { return 0 }
        let sorted = samples.sorted()
        let index = Int(Double(sorted.count - 1) * 0.95)
        return sorted[min(max(0, index), sorted.count - 1)]
    }
}
