import LumaCore

public enum SnippetsModuleBundle: ModuleBundle {
    public static var manifest: ModuleManifest { SnippetsModule.manifest }
    public static var warmupTier: WarmupTier { .hotPath }

    public static var detailMetadata: FeatureCard? {
        FeatureCard(
            id: .snippets,
            title: "Snippets",
            subtitle: "Cheatsheet library",
            icon: .symbol("text.cursor"),
            triggerKeyword: "s ",
            position: CardPosition(column: 1, row: 1),
            widgetStyle: WidgetCardStyle(symbolName: "text.cursor", topHex: "#34C759", bottomHex: "#248A3D")
        )
    }

    public static var presentation: ModulePresentation? {
        ModulePresentation(settingsSymbol: "text.cursor", listBadge: "s")
    }

    public static var commands: [CommandDefinition] {
        [
            CommandDefinition(
                id: "snippets",
                module: .snippets,
                title: "Snippets",
                primaryTrigger: "s",
                aliases: ["snip"],
                placeholder: "Find a snippet",
                usageFormat: "s / snip <query>",
                description: "Find snippets by title, tags, or content",
                examples: ["s git"],
                sectionTitle: "SNIPPETS",
                helpLines: [
                    "s — top snippets by frecency",
                    "s new <title> — create snippet and open editor",
                    "s git — fuzzy search title/tags/content",
                    "Return — copy snippet",
                    "Variables: {{uuid}} {{timestamp}} {{selection}} {{project}} {{file}} {{caret}}",
                    "s ? — this help"
                ],
                discoverPriority: 70
            )
        ]
    }

    public static func makeModule() -> any LumaModule { SnippetsModule() }
}
