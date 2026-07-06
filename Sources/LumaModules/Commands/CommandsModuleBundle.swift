import LumaCore

public enum CommandsModuleBundle: ModuleBundle {
    public static var manifest: ModuleManifest { CommandsModule.manifest }
    public static var warmupTier: WarmupTier { .onDemand }
    public static var defaultOffNote: String? {
        "Built-in shell commands plus user-defined scripts from commands.json (local executables, configurable timeout). Enable for global help discovery."
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
                    "exit — exit Luma (when Commands module is enabled)"
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
                id: "exit-luma",
                module: .commands,
                title: "Exit Luma",
                primaryTrigger: "exit",
                placeholder: "Exit Luma",
                usageFormat: "exit",
                description: "Exit Luma",
                examples: ["exit"],
                sectionTitle: "COMMANDS",
                helpLines: ["Exit Luma"],
                isDiscoverable: false
            )
        ]
    }

    public static func makeModule() -> any LumaModule { CommandsModule() }
}
