import LumaCore

public enum ProjectsModuleBundle: ModuleBundle {
    public static var manifest: ModuleManifest { ProjectsModule.manifest }
    public static var warmupTier: WarmupTier { .onDemand }
    public static var presentation: ModulePresentation? {
        ModulePresentation(settingsSymbol: "folder", listBadge: "proj")
    }

    public static var commands: [CommandDefinition] {
        [
            CommandDefinition(
                id: "projects",
                module: .projects,
                title: "Open Project",
                primaryTrigger: "p",
                aliases: ["proj", "project"],
                placeholder: "Open a project in Cursor, VS Code, Finder, or Terminal",
                usageFormat: "p / proj / project <name>",
                description: "Open a recent project in your preferred IDE or Finder",
                examples: ["p luma", "proj api"],
                sectionTitle: "PROJECTS",
                helpLines: [
                    "p — recent projects",
                    "p luma — open in preferred IDE",
                    "proj manage — pin, aliases, roots",
                    "p ? — this help"
                ],
                discoverPriority: 10
            )
        ]
    }

    public static func makeModule() -> any LumaModule { ProjectsModule() }
}
