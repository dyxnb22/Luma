import Foundation
import Testing
import LumaCore
@testable import LumaModules

@Test func simulatedUserTouchesEveryRequestedFeature() async throws {
    let appIndex = AppIndex(apps: [
        AppRecord(name: "Safari", bundleID: "com.apple.Safari", url: URL(fileURLWithPath: "/Applications/Safari.app"))
    ])
    #expect(appIndex.search("saf").first?.name == "Safari")

    let translate = TranslateModule()
    let queryContext = QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(60)))
    let translationResult = await translate.handle(Query(raw: "tr hello world", sequence: 1), context: queryContext)
    #expect(translationResult.items.first?.title == "Translate")

    let clipboard = ClipboardHistoryStore()
    await clipboard.add(text: "meeting notes", types: ["public.text"])
    #expect(await clipboard.search("meeting").first?.text == "meeting notes")

    let vault = SecretsVault()
    await vault.unlock()
    let secretID = try await vault.save(label: "OpenAI API Key", value: "sk-test")
    #expect(try await vault.revealValue(id: secretID) == "sk-test")

    let frame = WindowLayoutEngine.frame(for: .leftHalf, screen: CGRect(x: 0, y: 0, width: 1440, height: 900))
    #expect(frame.width == 720)

    let graph = NotesGraphIndexer.index(files: [
        "/vault/Luma.md": "Use [[Raycast Patterns]] #product"
    ])
    #expect(graph.edges.contains(NoteEdge(from: "/vault/Luma.md", to: "Raycast Patterns", kind: "wiki")))

    let review = ReviewScheduler.schedule(familiarity: .known, currentStage: 0, wrongCount: 0)
    #expect(review.stage == 1)

    let cards = FeatureCatalog.defaultCards()
    #expect(cards.count >= 6)
    #expect(Set(cards.map(\.position)).count == cards.count)
}

@Test func divergentUserInputsRemainSafe() async throws {
    let clipboard = ClipboardHistoryStore()
    await clipboard.add(text: "do not store", types: ["com.bitwarden.password"])
    #expect(await clipboard.search("").isEmpty)

    let vault = SecretsVault()
    do {
        _ = try await vault.searchLabels("")
        Issue.record("Locked vault should not search")
    } catch SecretsVaultError.locked {}

    let graph = NotesGraphIndexer.index(files: [
        "/vault/Weird.md": "#tag-one [[Unclosed link"
    ])
    #expect(graph.nodes.count == 1)
    #expect(graph.edges.allSatisfy { $0.kind != "wiki" })

    let missingApps = AppIndex(apps: []).search("Safari")
    #expect(missingApps.isEmpty)
}
