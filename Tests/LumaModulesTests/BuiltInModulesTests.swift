import Testing
@testable import LumaModules

@Test func builtInModulesAreRegistered() {
    #expect(BuiltInModules.makeAll().isEmpty == false)
}

@Test func activeBuiltInModulesExcludeDeferredWindows() {
    let ids = Set(BuiltInModules.makeAll().map { type(of: $0).manifest.identifier })
    #expect(!ids.contains(.windows))
    #expect(ids.contains(.translate))
    #expect(ids.contains(.clipboard))
    #expect(ids.contains(.notes))
    #expect(ids.contains(.todo))
    #expect(ids.contains(.wordbook))
    #expect(ids.contains(.snippets))
    #expect(ids.contains(.secrets))
    #expect(ids.contains(.media))
    #expect(ids.contains(.windowLayouts))
    #expect(ids.contains(.projects))
}

@Test func manifestCatalogMatchesRegisteredModuleOrder() {
    let registered = BuiltInModules.makeAll().map { type(of: $0).manifest.identifier }
    let catalog = BuiltInModules.manifestCatalog().map(\.identifier)
    #expect(catalog == registered)
}

@Test func deferredModulesRemainAvailableInSource() {
    let deferred = BuiltInModules.makeDeferred().map { type(of: $0).manifest.identifier }
    #expect(deferred.contains(.windows))
    #expect(deferred.count == 1)
}

@Test func builtInModulesOverridesReplaceSharedInstances() {
    let clipboard = ClipboardModule()
    let modules = BuiltInModules.makeAll(overrides: .init(clipboard: clipboard))
    let clipboardModules = modules.filter { type(of: $0).manifest.identifier == .clipboard }
    #expect(clipboardModules.count == 1)
    #expect((clipboardModules[0] as? ClipboardModule) === clipboard)
}

@Test func accessibilityDependentModulesAreDeclared() {
    #expect(BuiltInModules.accessibilityDependentModuleIDs.contains(.snippets))
    #expect(BuiltInModules.accessibilityDependentModuleIDs.contains(.windowLayouts))
    #expect(BuiltInModules.accessibilityDependentModuleIDs.contains(.menuItems))
    #expect(!BuiltInModules.accessibilityDependentModuleIDs.contains(.clipboard))
}

@Test func activeModulesIncludeAccessibilityDependentSnippets() {
    #expect(BuiltInModules.activeModulesRequireAccessibility() == true)
}

@Test func enabledModulesRequireAccessibilityRespectsDisabledSet() {
    let all = Set(BuiltInModules.makeAll().map { type(of: $0).manifest.identifier })
    #expect(BuiltInModules.enabledModulesRequireAccessibility(all) == true)
    #expect(
        BuiltInModules.enabledModulesRequireAccessibility(all.subtracting([.snippets, .windowLayouts, .menuItems])) == false
    )
}
