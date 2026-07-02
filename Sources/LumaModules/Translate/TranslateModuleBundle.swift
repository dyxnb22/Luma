import LumaCore

public enum TranslateModuleBundle: ModuleBundle {
    public static var manifest: ModuleManifest { TranslateModule.manifest }
    public static var warmupTier: WarmupTier { .hotPath }

    public static var detailMetadata: FeatureCard? {
        FeatureCard(
            id: .translate,
            title: "Translate",
            subtitle: "Translate text",
            icon: .symbol("character.bubble.fill"),
            triggerKeyword: "translate ",
            position: CardPosition(column: 0, row: 0),
            widgetStyle: WidgetCardStyle(symbolName: "character.bubble.fill", topHex: "#5AC8FA", bottomHex: "#0A84FF")
        )
    }

    public static var presentation: ModulePresentation? {
        ModulePresentation(settingsSymbol: "character.bubble", listBadge: "tr")
    }

    public static var commands: [CommandDefinition] {
        [
            CommandDefinition(
                id: "translate",
                module: .translate,
                title: "Translate",
                primaryTrigger: "tr",
                aliases: ["translate"],
                placeholder: "Text to translate",
                usageFormat: "tr / translate <text>",
                description: "Translate input text to your target language",
                examples: ["tr hello"],
                sectionTitle: "TRANSLATE",
                helpLines: [
                    "tr <text> — translate to target language",
                    "translate <text> — same command",
                    "help tr or tr ? — commands",
                    "Detail view: language chips, setup link to Translation Settings"
                ],
                discoverPriority: 30,
                bareBehavior: .openDetail
            )
        ]
    }

    public static func makeModule() -> any LumaModule { TranslateModule() }
}
