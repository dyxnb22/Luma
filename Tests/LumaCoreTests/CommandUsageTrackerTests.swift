import Foundation
import Testing
import LumaCore

@Test func commandUsageTrackerPersistsCounts() async throws {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent("luma-command-usage-\(UUID().uuidString).json")
    let tracker = CommandUsageTracker(url: url)
    defer { try? FileManager.default.removeItem(at: url) }

    await tracker.record(trigger: "rec")
    await tracker.record(trigger: "rec")
    await tracker.record(trigger: "tr")

    let snapshot = await tracker.snapshot()
    #expect(snapshot["rec"] == 2)
    #expect(snapshot["tr"] == 1)

    let reloaded = CommandUsageTracker(url: url)
    let again = await reloaded.snapshot()
    #expect(again["rec"] == 2)
}

@Test func discoverableCommandsSortByUsageBeforePriority() {
    let registry = BuiltInCommandRegistry.make()
    let usage = ["rec": 10, "p": 1]
    let sorted = registry.discoverableCommands(usage: usage)
    #expect(sorted.first?.primaryTrigger == "rec")
    #expect(sorted.contains { $0.primaryTrigger == "p" })
}

@Test func commandRouterOpenSettingsRoutesTargeted() {
    let router = CommandRouter()
    let commands = ModuleIdentifier(rawValue: "luma.commands")
    #expect(router.route(raw: "open-settings") == .targeted(module: commands, trigger: "open-settings", payload: ""))
    #expect(router.route(raw: "quit") == .targeted(module: commands, trigger: "quit", payload: ""))
}

@Test func globalHelpPrefersFrequentCommands() async {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent("luma-command-usage-help-\(UUID().uuidString).json")
    let tracker = CommandUsageTracker(url: url)
    defer { try? FileManager.default.removeItem(at: url) }

    await tracker.record(trigger: "rec")
    await tracker.record(trigger: "rec")
    await tracker.record(trigger: "rec")
    let usage = await tracker.snapshot()
    let rows = CommandEntryResults.globalHelp(registry: BuiltInCommandRegistry.make(), usage: usage)
    let commandRows = rows.filter { $0.id.key.hasPrefix("help.") && $0.id.key != "help.footer" }
    #expect(commandRows.first?.id.key == "help.rec")
}
