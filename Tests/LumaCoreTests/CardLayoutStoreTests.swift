import Foundation
import Testing
@testable import LumaCore

@Test func cardLayoutStorePersistsPositions() throws {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent(UUID().uuidString)
        .appendingPathComponent("layout.json")
    let store = CardLayoutStore(url: url)
    let module = ModuleIdentifier(rawValue: "luma.test")
    let cards = [
        FeatureCard(id: module, title: "Test", subtitle: "Card", icon: .none, position: CardPosition(column: 0, row: 0))
    ]

    try store.save(position: CardPosition(column: 2, row: 3), for: module)
    let loaded = store.load(cards: cards)
    #expect(loaded.first?.position == CardPosition(column: 2, row: 3))
}
