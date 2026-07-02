import LumaCore

public enum ClipboardModuleBundle: ModuleBundle {
    public static var manifest: ModuleManifest { ClipboardModule.manifest }
    public static var warmupTier: WarmupTier { .hotPath }

    public static var detailMetadata: FeatureCard? {
        FeatureCard(
            id: .clipboard,
            title: "Clipboard",
            subtitle: "Search clipboard",
            icon: .symbol("doc.on.clipboard.fill"),
            triggerKeyword: "clip ",
            position: CardPosition(column: 1, row: 0),
            widgetStyle: WidgetCardStyle(symbolName: "doc.on.clipboard.fill", topHex: "#FF9F0A", bottomHex: "#FF6B00")
        )
    }

    public static var presentation: ModulePresentation? {
        ModulePresentation(settingsSymbol: "doc.on.clipboard", listBadge: "clip")
    }

    public static var commands: [CommandDefinition] {
        [
            CommandDefinition(
                id: "clipboard",
                module: .clipboard,
                title: "Clipboard",
                primaryTrigger: "clip",
                aliases: ["cb"],
                placeholder: "Search clipboard history",
                usageFormat: "clip / cb <query>",
                description: "Search clipboard history",
                examples: ["clip jwt", "clip image"],
                sectionTitle: "CLIPBOARD",
                helpLines: [
                    "clip — open Clipboard in panel",
                    "clip https — filter links",
                    "help clip or clip ? — commands"
                ],
                discoverPriority: 40,
                bareBehavior: .openDetail
            )
        ]
    }

    public static func makeModule() -> any LumaModule { ClipboardModule() }
}
