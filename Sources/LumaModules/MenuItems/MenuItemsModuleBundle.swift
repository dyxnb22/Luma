import LumaCore

public enum MenuItemsModuleBundle: ModuleBundle {
    public static var manifest: ModuleManifest { MenuItemsModule.manifest }
    public static var warmupTier: WarmupTier { .onDemand }
    public static var presentation: ModulePresentation? {
        ModulePresentation(settingsSymbol: "menubar.rectangle", listBadge: "mb")
    }

    public static var commands: [CommandDefinition] {
        [
            CommandDefinition(
                id: "menu-items",
                module: .menuItems,
                title: "Menu Bar Search",
                primaryTrigger: "mb",
                aliases: ["menu"],
                placeholder: "Search the frontmost app menu",
                usageFormat: "mb / menu <menu item>",
                description: "Search and press a menu item in the frontmost app",
                examples: ["mb fold"],
                sectionTitle: "MENU",
                helpLines: [
                    "mb — recent cached menu items for frontmost app",
                    "mb fold — fuzzy search menu paths",
                    "menu <query> — same command",
                    "Requires Accessibility permission",
                    "mb ? — this help"
                ],
                discoverPriority: 35
            )
        ]
    }

    public static func makeModule() -> any LumaModule { MenuItemsModule() }
}
