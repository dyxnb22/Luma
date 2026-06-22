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
                id: .calculator,
                title: "Calculator",
                subtitle: "Quick math",
                icon: .symbol("function"),
                triggerKeyword: "=",
                position: CardPosition(column: 2, row: 0),
                widgetStyle: WidgetCardStyle(symbolName: "function", topHex: "#5856D6", bottomHex: "#3634A3")
            ),
            FeatureCard(
                id: .windows,
                title: "Windows",
                subtitle: "Focus windows",
                icon: .symbol("macwindow"),
                triggerKeyword: "win",
                position: CardPosition(column: 3, row: 0),
                widgetStyle: WidgetCardStyle(symbolName: "macwindow", topHex: "#30D158", bottomHex: "#248A3D")
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
