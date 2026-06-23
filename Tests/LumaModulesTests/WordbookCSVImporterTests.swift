import Foundation
import Testing
@testable import LumaModules

@Test func csvImporterParsesHeaderAndRows() {
    let csv = """
    term,phonetic,meaning,example,category
    hello,/həˈloʊ/,greeting,Hello world,greetings
    world,,earth,,places
    """
    let rows = WordbookCSVImporter.parse(csv)
    #expect(rows.count == 2)
    #expect(rows[0].term == "hello")
    #expect(rows[0].phonetic == "/həˈloʊ/")
    #expect(rows[0].meaning == "greeting")
    #expect(rows[1].term == "world")
}

@Test func csvImporterSupportsTabDelimiter() {
    let tsv = "word\tmeaning\napple\tfruit"
    let rows = WordbookCSVImporter.parse(tsv)
    #expect(rows.count == 1)
    #expect(rows[0].term == "apple")
    #expect(rows[0].phonetic == "fruit")
}

@Test func csvImporterUpsertsIntoStore() async throws {
    let (store, url) = try WordbookTestFixtures.makeStore()
    defer { try? FileManager.default.removeItem(at: url) }

    let parsed = WordbookCSVImporter.parse("term,meaning\nluma,light\nbeam,ray")
    let result = try await store.upsertWords(parsed)
    #expect(result.imported == 2)
    #expect(result.skipped == 0)

    let count = try await store.newWordCount()
    #expect(count == 2)

    let again = try await store.upsertWords(parsed)
    #expect(again.imported == 0)
    #expect(again.skipped == 2)
}
