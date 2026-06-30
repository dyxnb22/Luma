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

@Test func moduleRegistryQuitResolvesToCommandsNotKillProcess() {
    let registry = ModuleRegistry.makeCommandRegistry()
    let commands = ModuleIdentifier(rawValue: "luma.commands")
    #expect(registry.command(forTrigger: "quit")?.module == commands)
}

@Test func moduleRegistryDetailMetadataIncludesClipboard() {
    let cards = ModuleRegistry.moduleDetailMetadata()
    #expect(cards.contains { $0.id == .clipboard })
}

@Test func hotPathModulesMatchOnDemandExclusion() {
    #expect(ModuleRegistry.onDemandModuleIDs == [.notes, .projects, .menuItems])
    #expect(ModuleRegistry.hotPathModuleIDs.isDisjoint(with: ModuleRegistry.onDemandModuleIDs))
}
