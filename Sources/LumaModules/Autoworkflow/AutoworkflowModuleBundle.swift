import LumaCore

public enum AutoworkflowModuleBundle: ModuleBundle {
    public static var manifest: ModuleManifest { AutoworkflowModule.manifest }
    public static var warmupTier: WarmupTier { .onDemand }
    public static var presentation: ModulePresentation? {
        ModulePresentation(settingsSymbol: "gearshape.2", listBadge: "aw")
    }

    public static var defaultOffNote: String? {
        "Requires cc-loop installed and configured. Enable after verifying cc-loop works."
    }

    public static var commands: [CommandDefinition] {
        [
            CommandDefinition(
                id: "autoworkflow",
                module: .autoworkflow,
                title: "Auto Workflow",
                primaryTrigger: "aw",
                aliases: ["auto", "workflow"],
                placeholder: "Start or manage automated coding workflows",
                usageFormat: "aw / auto / workflow",
                description: "Manage AI coding automation with cc-loop",
                examples: ["aw", "auto status", "workflow start"],
                sectionTitle: "AUTOMATION",
                helpLines: [
                    "aw — open auto workflow panel",
                    "aw status — check workflow status",
                    "aw start — start a new workflow",
                    "aw ? — this help"
                ],
                discoverPriority: 90,
                bareBehavior: .openDetail
            )
        ]
    }

    public static func makeModule() -> any LumaModule {
        AutoworkflowModule()
    }
}
