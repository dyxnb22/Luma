import LumaCore

public enum WordbookModuleBundle: ModuleBundle {
    public static var manifest: ModuleManifest { WordbookModule.manifest }
    public static var warmupTier: WarmupTier { .hotPath }

    public static var detailMetadata: FeatureCard? {
        FeatureCard(
            id: .wordbook,
            title: "Wordbook",
            subtitle: "Vocabulary review",
            icon: .symbol("text.book.closed.fill"),
            triggerKeyword: "word ",
            position: CardPosition(column: 0, row: 1),
            widgetStyle: WidgetCardStyle(symbolName: "text.book.closed.fill", topHex: "#BF5AF2", bottomHex: "#5E5CE6")
        )
    }

    public static var presentation: ModulePresentation? {
        ModulePresentation(settingsSymbol: "text.book.closed", listBadge: "word")
    }

    public static var commands: [CommandDefinition] {
        [
            CommandDefinition(
                id: "wordbook",
                module: .wordbook,
                title: "Wordbook",
                primaryTrigger: "word",
                aliases: ["wb"],
                placeholder: "Search vocabulary or start review",
                usageFormat: "word / wb <query>",
                description: "Search vocabulary or start review",
                examples: ["word review"],
                sectionTitle: "WORDBOOK",
                helpLines: [
                    "word — open Wordbook in panel",
                    "word review — start review session",
                    "word abandon — search term/meaning",
                    "word ? — this help"
                ],
                discoverPriority: 100,
                bareBehavior: .openDetail
            )
        ]
    }

    public static func makeModule() -> any LumaModule { WordbookModule() }
}
