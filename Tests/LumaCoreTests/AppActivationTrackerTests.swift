import Foundation
import Testing
@testable import LumaCore

@Test func appActivationTrackerRanksRecentAndFrequentAppsHigher() async {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent(UUID().uuidString)
        .appendingPathComponent("app-activations.json")
    let now = Date()
    let tracker = AppActivationTracker(url: url, coalesceWindow: .zero)

    await tracker.record(bundleID: "com.stale", at: now.addingTimeInterval(-7200))
    await tracker.record(bundleID: "com.active", at: now)
    await tracker.record(bundleID: "com.active", at: now)
    await tracker.record(bundleID: "com.active", at: now)

    let ranked = await tracker.rankedBundleIDs(from: ["com.stale", "com.active"], at: now)
    #expect(ranked.first == "com.active")
}

@Test func appActivationTrackerPersistsAcrossRecreation() async {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent(UUID().uuidString)
        .appendingPathComponent("app-activations.json")

    let first = AppActivationTracker(url: url)
    await first.record(bundleID: "com.example.app", at: Date(timeIntervalSince1970: 100))
    await first.flush()

    let second = AppActivationTracker(url: url)
    let ranked = await second.rankedBundleIDs(from: ["com.example.app"], at: Date(timeIntervalSince1970: 200))
    #expect(ranked == ["com.example.app"])
}

@Test func appActivationTrackerCoalescesWritesUntilFlush() async {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent(UUID().uuidString)
        .appendingPathComponent("app-activations.json")

    let tracker = AppActivationTracker(url: url, coalesceWindow: .seconds(60))
    await tracker.record(bundleID: "com.a")
    await tracker.record(bundleID: "com.b")
    await tracker.record(bundleID: "com.c")

    #expect(!FileManager.default.fileExists(atPath: url.path))

    await tracker.flush()
    #expect(FileManager.default.fileExists(atPath: url.path))
}

@Test func appActivationTrackerZeroWindowPersistsImmediately() async {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent(UUID().uuidString)
        .appendingPathComponent("app-activations.json")

    let tracker = AppActivationTracker(url: url, coalesceWindow: .zero)
    await tracker.record(bundleID: "com.example")

    #expect(FileManager.default.fileExists(atPath: url.path))
}
