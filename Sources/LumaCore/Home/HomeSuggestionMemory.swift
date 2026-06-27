import Foundation

/// Lightweight memory for contextual home suggestions: avoid repeating the same row
/// within a short window, deprioritize recently completed create actions, and
/// suppress daily-note nudges right after opening one.
public actor HomeSuggestionMemory {
  public static let shared = HomeSuggestionMemory()

  public let repeatCooldown: TimeInterval
  public let dailyNoteCooldown: TimeInterval
  public let completedCooldown: TimeInterval

  private var recentlyShown: [String: Date] = [:]
  private var recentlyCompleted: [String: Date] = [:]
  private var dailyNoteOpenedAt: Date?
  private var sessionBoostKeys: Set<String> = []

  public init(
    repeatCooldown: TimeInterval = 120,
    dailyNoteCooldown: TimeInterval = 300,
    completedCooldown: TimeInterval = 300
  ) {
    self.repeatCooldown = repeatCooldown
    self.dailyNoteCooldown = dailyNoteCooldown
    self.completedCooldown = completedCooldown
  }

  public func recordShown(keys: [String]) {
    let now = Date()
    for key in keys {
      recentlyShown[key] = now
    }
    pruneStale(now: now)
  }

  public func recordCompleted(key: String) {
    let now = Date()
    recentlyCompleted[key] = now
    pruneStale(now: now)
  }

  public func recordDailyNoteOpened() {
    dailyNoteOpenedAt = Date()
    recordCompleted(key: "contextual.daily")
  }

  public func boostSessionContext(key: String) {
    sessionBoostKeys.insert(key)
  }

  public func clearSessionBoosts() {
    sessionBoostKeys.removeAll()
  }

  public func shouldSuppressSuggestion(key: String, now: Date = Date()) -> Bool {
    guard let last = recentlyShown[key] else { return false }
    return now.timeIntervalSince(last) < repeatCooldown
  }

  public func shouldSuppressDailyNoteSuggestion(now: Date = Date()) -> Bool {
    guard let opened = dailyNoteOpenedAt else { return false }
    return now.timeIntervalSince(opened) < dailyNoteCooldown
  }

  public func shouldDeprioritizeCompleted(key: String, now: Date = Date()) -> Bool {
    guard let last = recentlyCompleted[key] else { return false }
    return now.timeIntervalSince(last) < completedCooldown
  }

  /// Returns an adjusted priority for ranking home suggestions.
  public func adjustedPriority(
    base: Int,
    key: String,
    kind: HomeSuggestionKind,
    now: Date = Date()
  ) -> Int {
    var priority = base
    if sessionBoostKeys.contains(key) {
      priority += 6
    }
    switch kind {
    case .continueFlow:
      priority += 4
    case .create:
      if shouldDeprioritizeCompleted(key: key, now: now) {
        priority -= 25
      }
    case .transform, .utility:
      break
    }
    return priority
  }

  public func isEligible(key: String, kind: HomeSuggestionKind, now: Date = Date()) -> Bool {
    if shouldSuppressSuggestion(key: key, now: now) { return false }
    if key == "contextual.daily", shouldSuppressDailyNoteSuggestion(now: now) { return false }
    if kind == .create, shouldDeprioritizeCompleted(key: key, now: now) { return false }
    return true
  }

  private func pruneStale(now: Date) {
    recentlyShown = recentlyShown.filter { now.timeIntervalSince($0.value) < repeatCooldown * 2 }
    recentlyCompleted = recentlyCompleted.filter { now.timeIntervalSince($0.value) < completedCooldown * 2 }
  }
}
