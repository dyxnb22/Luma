import LumaCore
import LumaModules
import Testing

@Test func defaultEnabledModuleSetMatchesMVP() {
    let enabled = ModuleRegistry.defaultEnabledModuleIDs
    #expect(enabled.contains(.apps))
    #expect(enabled.contains(.clipboard))
    #expect(enabled.contains(.snippets))
    #expect(enabled.contains(.quicklinks))
    #expect(enabled.contains(.todo))
    #expect(enabled.contains(.translate))
    #expect(enabled.contains(.notes))
    #expect(!enabled.contains(.menuItems))
    #expect(!enabled.contains(.windowLayouts))
    #expect(!enabled.contains(.wordbook))
    #expect(!enabled.contains(.secrets))
    #expect(!enabled.contains(.killProcess))
    #expect(!enabled.contains(.projects))
    #expect(!enabled.contains(.media))
    #expect(!enabled.contains(.browserTabs))
    #expect(!enabled.contains(.commands))
}

@Test func freshInstallHomeGuideExcludesDefaultOffModules() {
    let registry = ModuleRegistry.makeCommandRegistry()
    let commands = registry.discoverableCommands
    let rows = HomeGuideCatalog.entryRows(
        from: commands,
        enabledModules: ModuleRegistry.defaultEnabledModuleIDs
    ) { $0 }
    let triggers = Set(rows.map(\.trigger))
    #expect(triggers.contains("app") || triggers.contains("apps"))
    #expect(!triggers.contains("rec"))
    #expect(!triggers.contains("tab"))
    #expect(!triggers.contains("mb"))
    #expect(!triggers.contains("wl"))
    #expect(!triggers.contains("sec"))
    #expect(!triggers.contains("p"))
}

@Test @MainActor func harnessDefaultOffPrefixYieldsNoModuleRows() async {
    let harness = await LauncherFlowHarness.makeWithBuiltInModules()
    harness.activatePanel()
    harness.type("mb fold")
    try? await Task.sleep(for: .milliseconds(300))
    let modules = Set(harness.lastSnapshot?.items.map(\.id.module) ?? [])
    #expect(!modules.contains(.menuItems))
}

@Test @MainActor func harnessDeactivateClearsInFlightQuery() async {
    let harness = await LauncherFlowHarness.makeWithBuiltInModules()
    harness.activatePanel()
    harness.type("clip test")
    harness.deactivatePanel()
    harness.activatePanel()
    harness.type("")
    try? await Task.sleep(for: .milliseconds(100))
    #expect(harness.queryText.isEmpty)
}
