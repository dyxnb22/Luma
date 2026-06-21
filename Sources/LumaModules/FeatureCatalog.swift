import LumaCore

public enum FeatureCatalog {
    public static func defaultCards() -> [FeatureCard] {
        [
            FeatureCard(id: .translate, title: "Translate", subtitle: "Translate selected, clipboard, or typed text", icon: .symbol("character.bubble"), position: CardPosition(column: 0, row: 0)),
            FeatureCard(id: .clipboard, title: "Clipboard", subtitle: "Search and pin clipboard history", icon: .symbol("doc.on.clipboard"), position: CardPosition(column: 1, row: 0)),
            FeatureCard(id: .secrets, title: "Secrets", subtitle: "Locked passwords, keys, and recovery codes", icon: .symbol("lock.shield"), position: CardPosition(column: 0, row: 1)),
            FeatureCard(id: .windowLayouts, title: "Layouts", subtitle: "Move and split app windows", icon: .symbol("rectangle.split.2x1"), position: CardPosition(column: 1, row: 1)),
            FeatureCard(id: .notes, title: "Notes", subtitle: "Markdown tree and knowledge graph", icon: .symbol("note.text"), position: CardPosition(column: 0, row: 2)),
            FeatureCard(id: .wordbook, title: "Wordbook", subtitle: "Review technical vocabulary", icon: .symbol("text.book.closed"), position: CardPosition(column: 1, row: 2))
        ]
    }
}
