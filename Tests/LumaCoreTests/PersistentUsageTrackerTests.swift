import Foundation
import Testing
@testable import LumaCore

@Test func persistentUsageTrackerSurvivesRecreation() async {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent(UUID().uuidString)
        .appendingPathComponent("usage.json")
    let id = ResultID(module: ModuleIdentifier(rawValue: "luma.test"), key: "item")

    let first = PersistentUsageTracker(url: url)
    await first.record(id, at: Date(timeIntervalSince1970: 100))

    let second = PersistentUsageTracker(url: url)
    let snapshot = await second.snapshot()
    #expect(snapshot[id]?.count == 1)
}
