import LumaCore

public enum CommandsModuleBundle: ModuleBundle {
    public static var manifest: ModuleManifest { CommandsModule.manifest }
    public static var warmupTier: WarmupTier { .hotPath }
    public static var defaultOffNote: String? {
        "Built-in shell commands (settings, reload, quit). Enable if you want them in global help discovery."
    }
    public static var presentation: ModulePresentation? {
        ModulePresentation(settingsSymbol: "terminal")
    }

    public static var commands: [CommandDefinition] {
        [
            CommandDefinition(
                id: "settings",
                module: .commands,
                title: "Settings",
                primaryTrigger: "settings",
                aliases: ["prefs"],
                placeholder: "Open Luma preferences",
                usageFormat: "settings / prefs",
                description: "Open Luma preferences",
                examples: ["settings"],
                sectionTitle: "COMMANDS",
                helpLines: [
                    "settings — open Luma preferences",
                    "open-settings — same from command mode",
                    "reload-modules — refresh module registry",
                    "quit — exit Luma"
                ],
                discoverPriority: 120,
                isDiscoverable: true
            ),
            CommandDefinition(
                id: "open-settings",
                module: .commands,
                title: "Open Settings",
                primaryTrigger: "open-settings",
                placeholder: "Open Luma preferences",
                usageFormat: "open-settings",
                description: "Open Luma preferences",
                examples: ["open-settings"],
                sectionTitle: "COMMANDS",
                helpLines: ["Open Luma preferences"],
                isDiscoverable: false
            ),
            CommandDefinition(
                id: "reload-modules",
                module: .commands,
                title: "Reload Modules",
                primaryTrigger: "reload-modules",
                placeholder: "Refresh module registry",
                usageFormat: "reload-modules",
                description: "Refresh the module registry and warm up modules",
                examples: ["reload-modules"],
                sectionTitle: "COMMANDS",
                helpLines: ["Reload module registry and warm up modules"],
                isDiscoverable: false
            ),
            CommandDefinition(
                id: "quit",
                module: .commands,
                title: "Quit Luma",
                primaryTrigger: "quit",
                placeholder: "Exit Luma",
                usageFormat: "quit",
                description: "Exit Luma",
                examples: ["quit"],
                sectionTitle: "COMMANDS",
                helpLines: ["Quit Luma"],
                isDiscoverable: false
            )
        ]
    }

    public static func makeModule() -> any LumaModule { CommandsModule() }
}
