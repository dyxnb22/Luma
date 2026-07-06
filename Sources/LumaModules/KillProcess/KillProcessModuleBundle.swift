import LumaCore

public enum KillProcessModuleBundle: ModuleBundle {
    public static var manifest: ModuleManifest { KillProcessModule.manifest }
    public static var warmupTier: WarmupTier { .hotPath }
    public static var defaultOffNote: String? {
        "Quit or force-kill the frontmost GUI app (`quit`, `kill`, `k`). Expert use — enable intentionally."
    }
    public static var presentation: ModulePresentation? {
        ModulePresentation(settingsSymbol: "xmark.circle", listBadge: "kill")
    }

    public static var commands: [CommandDefinition] {
        [
            CommandDefinition(
                id: "kill-process",
                module: .killProcess,
                title: "Kill Process",
                primaryTrigger: "kill",
                aliases: ["quit", "k"],
                placeholder: "Quit, force kill, or relaunch a running app",
                usageFormat: "kill / quit / k <app>",
                description: "Search GUI apps and quit, force kill, or relaunch",
                examples: ["kill Preview"],
                sectionTitle: "PROCESS",
                helpLines: [
                    "kill — recent running GUI apps",
                    "kill preview — search running apps",
                    "Return — standard quit",
                    "Tab — force kill or relaunch actions",
                    "help kill or kill ? — commands"
                ],
                discoverPriority: 45
            )
        ]
    }

    public static func makeModule() -> any LumaModule { KillProcessModule() }
}
