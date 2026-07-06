import Testing
import LumaCore
import LumaModules

@Test func moduleRegistryAggregatesAllBundles() {
    #expect(ModuleRegistry.allBundles.count == 16)
    #expect(ModuleRegistry.manifestCatalog().count == 16)
}

@Test func moduleRegistryCommandRegistryIncludesTranslate() {
    let registry = ModuleRegistry.makeCommandRegistry()
    let translate = ModuleIdentifier(rawValue: "luma.translate")
    #expect(registry.command(forTrigger: "tr")?.module == translate)
    #expect(registry.command(forTrigger: "translate")?.module == translate)
}

@Test func moduleRegistryQuitResolvesToKillProcessNotCommands() {
    let registry = ModuleRegistry.makeCommandRegistry()
    let killProcess = ModuleIdentifier(rawValue: "luma.kill-process")
    #expect(registry.command(forTrigger: "quit")?.module == killProcess)
}

@Test func moduleRegistryExitResolvesToCommandsWhenEnabled() {
    let registry = ModuleRegistry.makeCommandRegistry()
    let commands = ModuleIdentifier(rawValue: "luma.commands")
    #expect(registry.command(forTrigger: "exit")?.module == commands)
}

@Test func moduleRegistryDetailMetadataIncludesClipboard() {
    let cards = ModuleRegistry.moduleDetailMetadata()
    #expect(cards.contains { $0.id == .clipboard })
}

@Test func hotPathModulesMatchOnDemandExclusion() {
    #expect(ModuleRegistry.onDemandModuleIDs == [.notes, .projects, .menuItems, .media, .commands, .browserTabs])
    #expect(ModuleRegistry.hotPathModuleIDs.isDisjoint(with: ModuleRegistry.onDemandModuleIDs))
}

@Test func defaultPinnedModulesExcludeDefaultOff() {
    let defaultOff = Set(
        ModuleRegistry.manifestCatalog()
            .filter { !$0.defaultEnabled }
            .map(\.identifier)
    )
    let overlap = ModuleWarmupDefaults.defaultPinnedModuleIDs.intersection(defaultOff)
    #expect(overlap.isEmpty)
}

@Test func defaultOffModulesProvideOffNote() {
    for bundle in ModuleRegistry.allBundles where !bundle.manifest.defaultEnabled {
        #expect(
            ModuleRegistry.defaultOffNote(for: bundle.identifier) != nil,
            "Expected defaultOffNote for \(bundle.identifier.rawValue)"
        )
    }
}
