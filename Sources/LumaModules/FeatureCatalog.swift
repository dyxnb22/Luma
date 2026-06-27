import LumaCore

public enum FeatureCatalog {
    /// Visual metadata (gradients, triggers) for in-panel module detail headers under Route C.
    public static func moduleDetailMetadata() -> [FeatureCard] {
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
                triggerKeyword: "n ",
                position: CardPosition(column: 2, row: 0),
                widgetStyle: WidgetCardStyle(symbolName: "note.text", topHex: "#FFCC02", bottomHex: "#FFA000")
            ),
            FeatureCard(
                id: .todo,
                title: "Todo",
                subtitle: "Today + quick capture",
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
                triggerKeyword: "sec ",
                position: CardPosition(column: 2, row: 1),
                widgetStyle: WidgetCardStyle(symbolName: "lock.shield.fill", topHex: "#FFD60A", bottomHex: "#FF9F0A")
            ),
            FeatureCard(
                id: .quicklinks,
                title: "Quicklinks",
                subtitle: "URL templates",
                icon: .symbol("link"),
                triggerKeyword: "ql ",
                position: CardPosition(column: 3, row: 1),
                widgetStyle: WidgetCardStyle(symbolName: "link", topHex: "#30D158", bottomHex: "#0A7A35")
            )
        ]
    }

    @available(*, deprecated, renamed: "moduleDetailMetadata()")
    public static func dashboardCoreCards() -> [FeatureCard] {
        moduleDetailMetadata()
    }
}
