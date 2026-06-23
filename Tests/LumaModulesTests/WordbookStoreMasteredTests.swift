import Foundation
import Testing
@testable import LumaModules

@Test func masteredWordExcludedFromDueQueue() async throws {
    let (store, url) = try WordbookTestFixtures.makeStore()
    defer { try? FileManager.default.removeItem(at: url) }

    _ = try await store.upsertWords([WordbookTestFixtures.newWord(term: "alpha")])
    let words = try await store.allWords(limit: 1)
    guard let word = words.first else {
        Issue.record("Expected inserted word")
        return
    }

    _ = try await store.recordReview(wordID: word.id, familiarity: .mastered)
    let due = try await store.dueCount(before: WordbookDateFormat.iso(Date().addingTimeInterval(3600 * 24 * 365 * 10)))
    #expect(due == 0)
}
