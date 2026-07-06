import LumaCore
import Testing

@Test func homeGuideCatalogExcludesBuiltinCommandsAndDisabledModules() {
    let apps = ModuleIdentifier(rawValue: "luma.apps")
    let commands = ModuleIdentifier(rawValue: "luma.commands")
    let notes = ModuleIdentifier(rawValue: "luma.notes")
    let media = ModuleIdentifier(rawValue: "luma.media")
    let commandDefs = [
        CommandDefinition(
            id: "apps",
            module: apps,
            title: "Apps",
            primaryTrigger: "app",
            placeholder: "",
            isDiscoverable: true
        ),
        CommandDefinition(
            id: "settings",
            module: commands,
            title: "Settings",
            primaryTrigger: "settings",
            placeholder: ""
        ),
        CommandDefinition(
            id: "notes",
            module: notes,
            title: "Notes",
            primaryTrigger: "n",
            placeholder: "",
            discoverPriority: 50
        ),
        CommandDefinition(
            id: "media",
            module: media,
            title: "Media",
            primaryTrigger: "rec",
            placeholder: "",
            discoverPriority: 40,
            isDiscoverable: true
        )
    ]

    let enabled: Set<ModuleIdentifier> = [apps, notes]
    let rows = HomeGuideCatalog.entryRows(from: commandDefs, enabledModules: enabled) { $0 }
    #expect(rows.count == 2)
    #expect(rows.map(\.commandID).sorted() == ["apps", "notes"])
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

    let enabled: Set<ModuleIdentifier> = [
        ModuleIdentifier(rawValue: "luma.todo"),
        ModuleIdentifier(rawValue: "luma.notes")
    ]
    let rows = HomeGuideCatalog.entryRows(from: commands, enabledModules: enabled) { $0 }
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

    let enabled: Set<ModuleIdentifier> = [
        ModuleIdentifier(rawValue: "luma.todo"),
        ModuleIdentifier(rawValue: "luma.notes")
    ]
    let rows = HomeGuideCatalog.entryRows(from: commands, enabledModules: enabled) { $0 }
    #expect(rows.map(\.commandID) == ["notes-primary", "todo"])
    #expect(rows.filter { $0.moduleName == "Notes" }.count == 1)
}
