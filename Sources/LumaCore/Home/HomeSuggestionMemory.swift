import Foundation

/// Lightweight memory for contextual home suggestions: avoid repeating the same row
/// within a short window and deprioritize daily-note nudges right after opening one.
public actor HomeSuggestionMemory {
    public static let shared = HomeSuggestionMemory()

    public let repeatCooldown: TimeInterval
    public let dailyNoteCooldown: TimeInterval

    private var recentlyShown: [String: Date] = [:]
    private var dailyNoteOpenedAt: Date?

    public init(repeatCooldown: TimeInterval = 120, dailyNoteCooldown: TimeInterval = 300) {
        self.repeatCooldown = repeatCooldown
        self.dailyNoteCooldown = dailyNoteCooldown
    }

    public func recordShown(keys: [String]) {
        let now = Date()
        for key in keys {
            recentlyShown[key] = now
        }
        pruneStale(now: now)
    }

    public func recordDailyNoteOpened() {
        dailyNoteOpenedAt = Date()
    }

    public func shouldSuppressSuggestion(key: String, now: Date = Date()) -> Bool {
        guard let last = recentlyShown[key] else { return false }
        return now.timeIntervalSince(last) < repeatCooldown
    }

    public func shouldSuppressDailyNoteSuggestion(now: Date = Date()) -> Bool {
        guard let opened = dailyNoteOpenedAt else { return false }
        return now.timeIntervalSince(opened) < dailyNoteCooldown
    }

    private func pruneStale(now: Date) {
        recentlyShown = recentlyShown.filter { now.timeIntervalSince($0.value) < repeatCooldown * 2 }
    }
}
