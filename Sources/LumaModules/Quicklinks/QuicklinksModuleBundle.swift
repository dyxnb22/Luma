import LumaCore

public enum QuicklinksModuleBundle: ModuleBundle {
    public static var manifest: ModuleManifest { QuicklinksModule.manifest }
    public static var warmupTier: WarmupTier { .hotPath }

    public static var detailMetadata: FeatureCard? {
        FeatureCard(
            id: .quicklinks,
            title: "Quicklinks",
            subtitle: "URL templates",
            icon: .symbol("link"),
            triggerKeyword: "ql ",
            position: CardPosition(column: 3, row: 1),
            widgetStyle: WidgetCardStyle(symbolName: "link", topHex: "#30D158", bottomHex: "#0A7A35")
        )
    }

    public static var presentation: ModulePresentation? {
        ModulePresentation(settingsSymbol: "link", listBadge: "ql")
    }

    public static var commands: [CommandDefinition] {
        [
            CommandDefinition(
                id: "quicklinks",
                module: .quicklinks,
                title: "Quicklinks",
                primaryTrigger: "ql",
                aliases: ["quicklinks"],
                placeholder: "Manage URL templates or use gh/g/swift triggers",
                usageFormat: "ql / quicklinks",
                description: "Manage exact-trigger URL templates",
                examples: ["gh swift package", "ql"],
                sectionTitle: "QUICKLINKS",
                helpLines: [
                    "gh <query> — GitHub Search",
                    "g <query> — Google",
                    "swift <query> — Apple Developer",
                    "ql — manage Quicklinks",
                    "help ql or ql ? — commands"
                ],
                discoverPriority: 25,
                bareBehavior: .openDetail
            )
        ]
    }

    public static func makeModule() -> any LumaModule { QuicklinksModule() }
}
