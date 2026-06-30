import LumaCore

public enum AppsModuleBundle: ModuleBundle {
    public static var manifest: ModuleManifest { AppsModule.manifest }
    public static var warmupTier: WarmupTier { .hotPath }
    public static var presentation: ModulePresentation? {
        ModulePresentation(settingsSymbol: "app.badge", listBadge: "app")
    }

    public static var commands: [CommandDefinition] {
        [
            CommandDefinition(
                id: "apps",
                module: .apps,
                title: "Apps",
                primaryTrigger: "app",
                aliases: ["apps"],
                placeholder: "Search apps or type app top for memory usage",
                usageFormat: "app / apps <query>",
                description: "Launch or focus apps, or show memory usage leaders",
                examples: ["chrome", "app top"],
                sectionTitle: "APPS",
                helpLines: [
                    "Type app name — launch or focus",
                    "app top — memory usage leaders (quit from row)",
                    "app ? — this help"
                ],
                discoverPriority: 110,
                isDiscoverable: true,
                bareBehavior: .globalSearchShadow,
                bareReservedPayloads: ["top", "?", "help"]
            )
        ]
    }

    public static func makeModule() -> any LumaModule { AppsModule() }
}
