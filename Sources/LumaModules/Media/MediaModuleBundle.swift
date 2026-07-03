import LumaCore

public enum MediaModuleBundle: ModuleBundle {
    public static var manifest: ModuleManifest { MediaModule.manifest }
    public static var warmupTier: WarmupTier { .hotPath }
    public static var defaultOffNote: String? {
        "Personal media logbook. Enable when you want rec/log capture in search."
    }

    public static var presentation: ModulePresentation? {
        ModulePresentation(settingsSymbol: "film.stack", listBadge: "rec")
    }

    public static var commands: [CommandDefinition] {
        [
            CommandDefinition(
                id: "records",
                module: .media,
                title: "Log Record",
                primaryTrigger: "rec",
                aliases: ["record", "log", "m", "media"],
                placeholder: "Log a book, movie, show, anime, or game",
                usageFormat: "rec / record / log / m / media <query>",
                description: "Log or search books, movies, shows, anime, and games",
                examples: ["rec 三体 book done 9 #sci-fi"],
                sectionTitle: "RECORDS",
                helpLines: [
                    "rec — recent items + open logbook",
                    "rec log — full Records view",
                    "rec 三体 book done 9 #sci-fi — quick capture DSL",
                    "rec 三体 — search or partial capture",
                    "help rec or rec ? — commands"
                ],
                discoverPriority: 80,
                bareBehavior: .openDetail
            )
        ]
    }

    public static func makeModule() -> any LumaModule { MediaModule() }
}
