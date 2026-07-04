import LumaCore
import Testing

@Test func homeGuideCatalogExcludesAppsAndBuiltinCommands() {
    let commands = [
        CommandDefinition(
            id: "apps",
            module: ModuleIdentifier(rawValue: "luma.apps"),
            title: "Apps",
            primaryTrigger: "app",
            placeholder: ""
        ),
        CommandDefinition(
            id: "settings",
            module: ModuleIdentifier(rawValue: "luma.commands"),
            title: "Settings",
            primaryTrigger: "settings",
            placeholder: ""
        ),
        CommandDefinition(
            id: "notes",
            module: ModuleIdentifier(rawValue: "luma.notes"),
            title: "Notes",
            primaryTrigger: "n",
            placeholder: "",
            discoverPriority: 50
        )
    ]

    let rows = HomeGuideCatalog.entryRows(from: commands) { $0 }
    #expect(rows.count == 1)
    #expect(rows[0].commandID == "notes")
    #expect(rows[0].trigger == "n")
}

@Test func homeGuideCatalogSortsByDiscoverPriority() {
    let commands = [
        CommandDefinition(
            id: "todo",
            module: ModuleIdentifier(rawValue: "luma.todo"),
            title: "Todo",
            primaryTrigger: "t",
            placeholder: "",
            discoverPriority: 10
        ),
        CommandDefinition(
            id: "notes",
            module: ModuleIdentifier(rawValue: "luma.notes"),
            title: "Notes",
            primaryTrigger: "n",
            placeholder: "",
            discoverPriority: 90
        )
    ]

    let rows = HomeGuideCatalog.entryRows(from: commands) { $0 }
    #expect(rows.map(\.commandID) == ["notes", "todo"])
}

@Test func homeGuideCatalogKeepsOnePrimaryRowPerModule() {
    let module = ModuleIdentifier(rawValue: "luma.notes")
    let commands = [
        CommandDefinition(
            id: "notes-secondary",
            module: module,
            title: "Notes Search",
            primaryTrigger: "note",
            placeholder: "",
            discoverPriority: 10
        ),
        CommandDefinition(
            id: "notes-primary",
            module: module,
            title: "Notes",
            primaryTrigger: "n",
            placeholder: "",
            discoverPriority: 90
        ),
        CommandDefinition(
            id: "todo",
            module: ModuleIdentifier(rawValue: "luma.todo"),
            title: "Todo",
            primaryTrigger: "t",
            placeholder: "",
            discoverPriority: 50
        )
    ]

    let rows = HomeGuideCatalog.entryRows(from: commands) { $0 }
    #expect(rows.map(\.commandID) == ["notes-primary", "todo"])
    #expect(rows.filter { $0.moduleName == "Notes" }.count == 1)
}
