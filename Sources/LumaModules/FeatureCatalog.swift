import LumaCore

public enum FeatureCatalog {
    public static func dashboardCoreCards() -> [FeatureCard] {
        [
            FeatureCard(
                id: .translate,
                title: "Translate",
                subtitle: "Translate text",
                icon: .symbol("character.bubble.fill"),
                triggerKeyword: "translate ",
                position: CardPosition(column: 0, row: 0),
                widgetStyle: WidgetCardStyle(symbolName: "character.bubble.fill", topHex: "#5AC8FA", bottomHex: "#0A84FF")
            ),
            FeatureCard(
                id: .clipboard,
                title: "Clipboard",
                subtitle: "Search clipboard",
                icon: .symbol("doc.on.clipboard.fill"),
                triggerKeyword: "clip ",
                position: CardPosition(column: 1, row: 0),
                widgetStyle: WidgetCardStyle(symbolName: "doc.on.clipboard.fill", topHex: "#FF9F0A", bottomHex: "#FF6B00")
            ),
            FeatureCard(
                id: .notes,
                title: "Notes",
                subtitle: "Markdown notes",
                icon: .symbol("note.text"),
                triggerKeyword: "note ",
                position: CardPosition(column: 2, row: 0),
                widgetStyle: WidgetCardStyle(symbolName: "note.text", topHex: "#FFCC02", bottomHex: "#FFA000")
            ),
            FeatureCard(
                id: .todo,
                title: "Todo",
                subtitle: "Reminders capture",
                icon: .symbol("checkmark.circle.fill"),
                triggerKeyword: "t ",
                position: CardPosition(column: 3, row: 0),
                widgetStyle: WidgetCardStyle(symbolName: "checkmark.circle.fill", topHex: "#FF375F", bottomHex: "#D70015")
            ),
            FeatureCard(
                id: .wordbook,
                title: "Wordbook",
                subtitle: "Vocabulary review",
                icon: .symbol("text.book.closed.fill"),
                triggerKeyword: "word ",
                position: CardPosition(column: 0, row: 1),
                widgetStyle: WidgetCardStyle(symbolName: "text.book.closed.fill", topHex: "#BF5AF2", bottomHex: "#5E5CE6")
            ),
            FeatureCard(
                id: .snippets,
                title: "Snippets",
                subtitle: "Cheatsheet library",
                icon: .symbol("text.cursor"),
                triggerKeyword: "s ",
                position: CardPosition(column: 1, row: 1),
                widgetStyle: WidgetCardStyle(symbolName: "text.cursor", topHex: "#34C759", bottomHex: "#248A3D")
            ),
            FeatureCard(
                id: .secrets,
                title: "Secrets",
                subtitle: "Developer vault",
                icon: .symbol("lock.shield.fill"),
                triggerKeyword: "secret ",
                position: CardPosition(column: 2, row: 1),
                widgetStyle: WidgetCardStyle(symbolName: "lock.shield.fill", topHex: "#FFD60A", bottomHex: "#FF9F0A")
            )
        ]
    }

    public static func defaultCards() -> [FeatureCard] {
        [
            FeatureCard(id: .translate, title: "Translate", subtitle: "Translate text", icon: .symbol("character.bubble"), triggerKeyword: "translate ", position: CardPosition(column: 0, row: 0)),
            FeatureCard(id: .clipboard, title: "Clipboard", subtitle: "Search clipboard", icon: .symbol("doc.on.clipboard"), triggerKeyword: "clip ", position: CardPosition(column: 1, row: 0)),
            FeatureCard(id: .secrets, title: "Secrets", subtitle: "Vault search", icon: .symbol("lock.shield"), triggerKeyword: "secret ", position: CardPosition(column: 2, row: 0)),
            FeatureCard(id: .windowLayouts, title: "Layouts", subtitle: "Window layouts", icon: .symbol("rectangle.split.2x1"), triggerKeyword: "win ", position: CardPosition(column: 3, row: 0)),
            FeatureCard(id: .notes, title: "Notes", subtitle: "Markdown notes", icon: .symbol("note.text"), triggerKeyword: "note ", position: CardPosition(column: 4, row: 0)),
            FeatureCard(id: .wordbook, title: "Wordbook", subtitle: "Vocabulary review", icon: .symbol("text.book.closed"), triggerKeyword: "word ", position: CardPosition(column: 5, row: 0))
        ]
    }
}
