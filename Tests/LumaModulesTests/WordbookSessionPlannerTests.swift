import Foundation
import Testing
@testable import LumaModules

@Test func sessionPlannerNewWordsOnlyReturnsFreshCard() async throws {
    let (store, url) = try WordbookTestFixtures.makeStore()
    defer { try? FileManager.default.removeItem(at: url) }

    _ = try await store.upsertWords([
        WordbookTestFixtures.newWord(term: "alpha"),
        WordbookTestFixtures.newWord(term: "beta")
    ])

    let planner = WordbookSessionPlanner(store: store)
    await planner.startNewSession(newWordsOnly: true)

    let first = try await planner.nextCard()
    if case .fresh(let word) = first {
        #expect(["alpha", "beta"].contains(word.term))
    } else {
        Issue.record("Expected fresh card in new-words-only mode")
    }
    let stats = await planner.sessionStats()
    #expect(stats.learned == 1)
    #expect(stats.reviewed == 0)
}

@Test func sessionPlannerReturnsReviewForDueWord() async throws {
    let (store, url) = try WordbookTestFixtures.makeStore()
    defer { try? FileManager.default.removeItem(at: url) }

    let past = WordbookDateFormat.iso(Date().addingTimeInterval(-3600))
    try WordbookTestFixtures.insertDueWord(at: url, term: "due-word", nextReviewAt: past)

    let planner = WordbookSessionPlanner(store: store)
    await planner.startNewSession()

    let card = try await planner.nextCard()
    if case .review(let word) = card {
        #expect(word.term == "due-word")
        let stats = await planner.sessionStats()
        #expect(stats.reviewed == 1)
    } else {
        Issue.record("Expected review card for due word")
    }
}
