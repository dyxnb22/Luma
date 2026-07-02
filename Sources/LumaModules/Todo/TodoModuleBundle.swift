import LumaCore

public enum TodoModuleBundle: ModuleBundle {
    public static var manifest: ModuleManifest { TodoModule.manifest }
    public static var warmupTier: WarmupTier { .hotPath }

    public static var detailMetadata: FeatureCard? {
        FeatureCard(
            id: .todo,
            title: "Todo",
            subtitle: "Today + quick capture",
            icon: .symbol("checkmark.circle.fill"),
            triggerKeyword: "t ",
            position: CardPosition(column: 3, row: 0),
            widgetStyle: WidgetCardStyle(symbolName: "checkmark.circle.fill", topHex: "#FF375F", bottomHex: "#D70015")
        )
    }

    public static var presentation: ModulePresentation? {
        ModulePresentation(settingsSymbol: "checkmark.circle", listBadge: "t")
    }

    public static var commands: [CommandDefinition] {
        [
            CommandDefinition(
                id: "todo",
                module: .todo,
                title: "Todo",
                primaryTrigger: "t",
                aliases: ["todo"],
                placeholder: "Add a task or list today's reminders",
                usageFormat: "t / todo <task>",
                description: "List today's reminders or create a new task",
                examples: ["t buy milk tomorrow"],
                sectionTitle: "TODO",
                helpLines: [
                    "t — open Todo in panel (today's due list)",
                    "t buy milk — create reminder (Inbox when no date)",
                    "t pay rent tomorrow 9:00 — create with due date",
                    "Return on a row — mark complete",
                    "help todo or t ? — commands"
                ],
                discoverPriority: 50,
                bareBehavior: .openDetail
            )
        ]
    }

    public static func makeModule() -> any LumaModule { TodoModule() }
}
