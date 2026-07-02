import LumaCore

public enum SecretsModuleBundle: ModuleBundle {
    public static var manifest: ModuleManifest { SecretsModule.manifest }
    public static var warmupTier: WarmupTier { .hotPath }

    public static var detailMetadata: FeatureCard? {
        FeatureCard(
            id: .secrets,
            title: "Secrets",
            subtitle: "Developer vault",
            icon: .symbol("lock.shield.fill"),
            triggerKeyword: "sec ",
            position: CardPosition(column: 2, row: 1),
            widgetStyle: WidgetCardStyle(symbolName: "lock.shield.fill", topHex: "#FFD60A", bottomHex: "#FF9F0A")
        )
    }

    public static var presentation: ModulePresentation? {
        ModulePresentation(settingsSymbol: "lock.shield", listBadge: "sec")
    }

    public static var commands: [CommandDefinition] {
        [
            CommandDefinition(
                id: "secrets",
                module: .secrets,
                title: "Secrets",
                primaryTrigger: "sec",
                aliases: ["secret", "secrets"],
                placeholder: "Search saved secrets",
                usageFormat: "sec / secret / secrets <query>",
                description: "Search saved secrets in the vault",
                examples: ["sec aws"],
                sectionTitle: "SECRETS",
                helpLines: [
                    "sec unlock — unlock vault",
                    "sec aws — search by label",
                    "Return — copy secret (auto-clear pasteboard)",
                    "help secret or sec ? — commands"
                ],
                discoverPriority: 90,
                bareBehavior: .openDetail
            )
        ]
    }

    public static func makeModule() -> any LumaModule { SecretsModule() }
}
