import LumaCore

public enum WindowLayoutsModuleBundle: ModuleBundle {
    public static var manifest: ModuleManifest { WindowLayoutsModule.manifest }
    public static var warmupTier: WarmupTier { .hotPath }
    public static var presentation: ModulePresentation? {
        ModulePresentation(settingsSymbol: "rectangle.split.2x1", listBadge: "win")
    }

    public static var commands: [CommandDefinition] {
        [
            CommandDefinition(
                id: "window-layouts",
                module: .windowLayouts,
                title: "Move Window",
                primaryTrigger: "win",
                aliases: ["wl", "layout"],
                placeholder: "Move focused window: left, right, max, center",
                usageFormat: "win / wl / layout <position>",
                description: "Move the focused window left, right, maximized, or centered",
                examples: ["win left", "wl center"],
                sectionTitle: "WINDOWS",
                helpLines: [
                    "win left / right / max / center — move focused window",
                    "wl and layout are aliases",
                    "win ? — this help"
                ],
                discoverPriority: 20
            )
        ]
    }

    public static func makeModule() -> any LumaModule { WindowLayoutsModule() }
}
