import Foundation
import Testing
@testable import LumaModules

@Test func csvExportRoundTripPreservesCount() async throws {
    let (store, url) = try WordbookTestFixtures.makeStore()
    defer { try? FileManager.default.removeItem(at: url) }

    let source = """
    term,phonetic,meaning,example,category
    hello,/həˈloʊ/,greeting,Hello world,greetings
    world,,earth,,places
    """
    let parsed = WordbookCSVImporter.parse(source)
    _ = try await store.upsertWords(parsed)

    let all = try await store.allWords(limit: 100, offset: 0)
    let exported = WordbookCSVImporter.export(all)
    let reimported = WordbookCSVImporter.parse(exported)
    #expect(reimported.count == 2)
    #expect(reimported.map(\.term).sorted() == ["hello", "world"])
}

@Test func csvExportEscapesCommasAndQuotes() {
    let entry = WordEntry(
        id: 1,
        term: "test",
        phonetic: "",
        meaning: "a, b",
        example: "say \"hi\"",
        category: "",
        familiarity: "new",
        reviewStage: 0,
        reviewCount: 0,
        wrongCount: 0,
        nextReviewAt: ""
    )
    let csv = WordbookCSVImporter.export([entry])
    let rows = WordbookCSVImporter.parse(csv)
    #expect(rows.count == 1)
    #expect(rows[0].meaning == "a, b")
    #expect(rows[0].example == "say \"hi\"")
}
