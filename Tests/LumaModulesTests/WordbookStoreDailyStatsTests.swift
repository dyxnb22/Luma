import Foundation
import Testing
@testable import LumaModules

@Test func dailyStatsResetOnNewDay() async throws {
    let (store, url) = try WordbookTestFixtures.makeStore()
    defer { try? FileManager.default.removeItem(at: url) }

    let yesterday = Calendar.current.date(byAdding: .day, value: -1, to: Date())!
    let yesterdayKey = {
        let f = DateFormatter()
        f.dateFormat = "yyyy-MM-dd"
        f.timeZone = .current
        return f.string(from: yesterday)
    }()

    try await store.setSetting("daily_stats_date", value: yesterdayKey)
    try await store.setIntSetting("daily_new_seen", value: 5)
    try await store.setIntSetting("daily_wrong_count", value: 3)

    try await store.resetDailyStatsIfNeeded(now: Date())

    let seen = try await store.intSetting("daily_new_seen", default: -1)
    let wrong = try await store.intSetting("daily_wrong_count", default: -1)
    #expect(seen == 0)
    #expect(wrong == 0)
}

@Test func dailyNewRemainingDecreasesAfterShown() async throws {
    let (store, url) = try WordbookTestFixtures.makeStore()
    defer { try? FileManager.default.removeItem(at: url) }

    let before = try await store.dailyNewRemaining(dueCount: 0)
    try await store.recordNewWordShown()
    let after = try await store.dailyNewRemaining(dueCount: 0)
    #expect(after == before - 1)
}

@Test func dailyNewLimitScalesWithDueCount() async throws {
    let (store, url) = try WordbookTestFixtures.makeStore()
    defer { try? FileManager.default.removeItem(at: url) }

    let lowDue = try await store.dailyNewLimitForDueCount(10)
    let highDue = try await store.dailyNewLimitForDueCount(90)
    #expect(lowDue > highDue)
}
