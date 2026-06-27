import Foundation
import Testing
import LumaCore

@Test func crossModuleActionTitlesAreStable() {
  #expect(CrossModuleActionTitles.appendToNote == "Append to Note")
  #expect(CrossModuleActionTitles.createSnippet == "Create Snippet")
  #expect(CrossModuleActionTitles.addQuicklink == "Add Quicklink")
}

@Test func lumaDiagnosticsFlagsMissingNotesRoot() {
  let manifests = [
    ModuleManifest(
      identifier: ModuleIdentifier(rawValue: "luma.notes"),
      displayName: "Notes",
      capabilities: [.queryable],
      defaultEnabled: true,
      priority: 1,
      queryTimeout: .milliseconds(20)
    )
  ]
  let summary = LumaDiagnostics.summarize(manifests: manifests, notesRootConfigured: false)
  #expect(summary.issues.contains { $0.moduleID?.rawValue == "luma.notes" })
  #expect(summary.issues.contains { $0.message.contains("Notes root") })
}

@Test func lumaDiagnosticsReportsModuleCounts() {
  let manifests = [
    ModuleManifest(
      identifier: ModuleIdentifier(rawValue: "luma.notes"),
      displayName: "Notes",
      capabilities: [.queryable],
      defaultEnabled: true,
      priority: 1,
      queryTimeout: .milliseconds(20)
    ),
    ModuleManifest(
      identifier: ModuleIdentifier(rawValue: "luma.todo"),
      displayName: "Todo",
      capabilities: [.queryable],
      defaultEnabled: false,
      priority: 2,
      queryTimeout: .milliseconds(20)
    )
  ]
  let summary = LumaDiagnostics.summarize(manifests: manifests, notesRootConfigured: true)
  #expect(summary.moduleCount == 2)
  #expect(summary.defaultEnabledCount == 1)
  #expect(summary.isHealthy)
}
