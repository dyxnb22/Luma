import LumaCore

public enum BrowserTabsModuleBundle: ModuleBundle {
    public static var manifest: ModuleManifest { BrowserTabsModule.manifest }
    public static var warmupTier: WarmupTier { .hotPath }
    public static var defaultOffNote: String? {
        "Requires Automation permission per browser. Enable only if you search open tabs regularly."
    }
    public static var presentation: ModulePresentation? {
        ModulePresentation(settingsSymbol: "safari", listBadge: "tab")
    }

    public static var commands: [CommandDefinition] {
        [
            CommandDefinition(
                id: "browser-tabs",
                module: .browserTabs,
                title: "Browser Tabs",
                primaryTrigger: "tab",
                aliases: ["tabs"],
                placeholder: "Search open browser tabs",
                usageFormat: "tab / tabs <query>",
                description: "Search cached Safari, Chrome, Brave, Edge, and Arc tabs",
                examples: ["tab github"],
                sectionTitle: "TABS",
                helpLines: [
                    "tab — cached browser tabs",
                    "tab github — search title and URL",
                    "Requires Automation permission per browser",
                    "Disabled by default; enable in Settings → Modules",
                    "tab ? — this help"
                ],
                discoverPriority: 55
            )
        ]
    }

    public static func makeModule() -> any LumaModule { BrowserTabsModule() }
}
