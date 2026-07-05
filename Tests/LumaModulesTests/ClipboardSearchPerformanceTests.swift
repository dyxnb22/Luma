import Foundation
import Testing
@testable import LumaModules

private func p95(_ samples: [Double]) -> Double {
    let sorted = samples.sorted()
    return sorted[Int(Double(sorted.count - 1) * 0.95)]
}

@Test func clipboardSearchWithFullHistoryStaysUnderBudget() async {
    let store = ClipboardHistoryStore(maxEntries: 500, persistenceURL: nil)
    for index in 0..<500 {
        await store.add(
            text: "clipboard entry \(index) token\(index % 17)",
            types: ["public.utf8-plain-text"],
            now: Date(timeIntervalSince1970: Double(index))
        )
    }

    var samples: [Double] = []
    for index in 0..<100 {
        let start = ContinuousClock.now
        _ = await store.search("token3", limit: 20)
        let elapsed = start.duration(to: .now)
        samples.append(
            Double(elapsed.components.seconds) * 1000
                + Double(elapsed.components.attoseconds) / 1_000_000_000_000_000
        )
        _ = index
    }

    #expect(p95(samples) < 10)
}
