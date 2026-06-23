import Foundation

public actor WordbookSessionPlanner {
    private let store: WordbookStore
    private var mixedReviewShown = 0
    private var mixedNewShown = 0
    private var dueSessionCutoff: String?
    private var newWordsOnly = false

    public enum Card: Sendable {
        case review(WordEntry)
        case fresh(WordEntry)
        case done(reviewed: Int, learned: Int)
    }

    public init(store: WordbookStore) {
        self.store = store
    }

    public func startNewSession(newWordsOnly: Bool = false) {
        mixedReviewShown = 0
        mixedNewShown = 0
        self.newWordsOnly = newWordsOnly
        dueSessionCutoff = WordbookDateFormat.iso(Date())
    }

    public func sessionStats() -> (reviewed: Int, learned: Int) {
        (mixedReviewShown, mixedNewShown)
    }

    public func nextCard() async throws -> Card {
        if dueSessionCutoff == nil { startNewSession() }
        let cutoff = dueSessionCutoff!

        if newWordsOnly {
            let stats = try await store.sessionPlanningStats(cutoff: cutoff)
            if stats.quotaLeft > 0, let word = try await store.nextNewWord() {
                mixedNewShown += 1
                try await store.recordNewWordShown()
                return .fresh(word)
            }
            return .done(reviewed: mixedReviewShown, learned: mixedNewShown)
        }

        let stats = try await store.sessionPlanningStats(cutoff: cutoff)
        let dueLeft = stats.dueLeft
        let newLeft = stats.newLeft
        let quotaLeft = stats.quotaLeft
        let wrongToday = stats.wrongToday

        var target = dueLeft > 80 ? 15 : (dueLeft > 30 ? 25 : 35)
        if wrongToday >= 5 { target = max(8, target / 2) }

        let shownTotal = mixedReviewShown + mixedNewShown
        let currentRatio = shownTotal > 0 ? Double(mixedNewShown) / Double(shownTotal) : 0
        let lower = target <= 15 ? 0.10 : (target <= 25 ? 0.18 : 0.26)
        let upper = target <= 15 ? 0.22 : (target <= 25 ? 0.34 : 0.45)

        let canIntroduce = newLeft > 0 && quotaLeft > 0
        var shouldIntroduce = canIntroduce && dueLeft > 0 && Int.random(in: 0..<100) < target
        if canIntroduce && dueLeft > 0 && shownTotal >= 5 && currentRatio < lower { shouldIntroduce = true }
        if canIntroduce && dueLeft > 0 && currentRatio > upper { shouldIntroduce = false }

        if let word = shouldIntroduce ? try await store.nextNewWord() : try await store.nextDueWord(before: cutoff) {
            if shouldIntroduce {
                mixedNewShown += 1
                try await store.recordNewWordShown()
                return .fresh(word)
            } else {
                mixedReviewShown += 1
                return .review(word)
            }
        }
        if dueLeft > 0, let w = try await store.nextDueWord(before: cutoff) {
            mixedReviewShown += 1
            return .review(w)
        }
        if canIntroduce, let w = try await store.nextNewWord() {
            mixedNewShown += 1
            try await store.recordNewWordShown()
            return .fresh(w)
        }
        return .done(reviewed: mixedReviewShown, learned: mixedNewShown)
    }
}
