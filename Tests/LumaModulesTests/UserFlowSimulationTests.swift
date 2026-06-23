import Foundation
import Testing
import LumaCore
@testable import LumaModules

@Test func simulatedUserTouchesEveryRequestedFeature() async throws {
    let appIndex = AppIndex(apps: [
        AppRecord(bundleID: "com.apple.Safari", url: URL(fileURLWithPath: "/Applications/Safari.app"), name: "Safari")
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

    let notesIndex = NotesTreeIndex()
    let notesRoot = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: notesRoot, withIntermediateDirectories: true)
    try "Use [[Raycast Patterns]]".write(to: notesRoot.appendingPathComponent("Luma.md"), atomically: true, encoding: .utf8)
    await notesIndex.setRoot(notesRoot)
    await notesIndex.warmup()
    #expect(await notesIndex.search(fuzzy: "luma").first?.name == "Luma")
    try? FileManager.default.removeItem(at: notesRoot)

    let review = ReviewScheduler.schedule(familiarity: .known, currentStage: 0, wrongCount: 0)
    #expect(review.stage == 1)

    let cards = FeatureCatalog.dashboardCoreCards()
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

    let notesIndex = NotesTreeIndex()
    #expect(await notesIndex.search(fuzzy: "weird").isEmpty)

    let missingApps = AppIndex(apps: []).search("Safari")
    #expect(missingApps.isEmpty)
}
