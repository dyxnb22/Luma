import LumaCore

public enum NotesModuleBundle: ModuleBundle {
    public static var manifest: ModuleManifest { NotesModule.manifest }
    public static var warmupTier: WarmupTier { .onDemand }

    public static var detailMetadata: FeatureCard? {
        FeatureCard(
            id: .notes,
            title: "Notes",
            subtitle: "Markdown notes",
            icon: .symbol("note.text"),
            triggerKeyword: "n ",
            position: CardPosition(column: 2, row: 0),
            widgetStyle: WidgetCardStyle(symbolName: "note.text", topHex: "#FFCC02", bottomHex: "#FFA000")
        )
    }

    public static var presentation: ModulePresentation? {
        ModulePresentation(settingsSymbol: "note.text", listBadge: "notes")
    }

    public static var commands: [CommandDefinition] {
        [
            CommandDefinition(
                id: "notes",
                module: .notes,
                title: "Notes",
                primaryTrigger: "n",
                aliases: ["note", "notes"],
                placeholder: "New note, daily, or search by filename",
                usageFormat: "n / note / notes <query>",
                description: "Search notes or create and open notes",
                examples: ["n daily", "n new idea"],
                sectionTitle: "NOTES",
                helpLines: [
                    "n — open Notes in panel",
                    "n <query> — fuzzy find by filename",
                    "n new <title> — create in Inbox and open",
                    "n daily — open or create today's daily note",
                    "n cap <text> — append bullet to today's daily note",
                    "n review week — weekly review with modified notes",
                    "help note or n ? — commands"
                ],
                discoverPriority: 60,
                bareBehavior: .openDetail
            )
        ]
    }

    public static func makeModule() -> any LumaModule { NotesModule() }
}
