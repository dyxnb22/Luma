import Foundation
import Testing
import LumaCore

@Test func projectAwareHomeContextSuppressesDisabledSnippetModule() {
    let gate = HomeEnablementGate()
    gate.update([.workbenchNotes, .workbenchProjects])
    let context = HomeContributionContext(
        pinnedModuleIDs: [.workbenchSnippets],
        enabledModuleIDs: gate.snapshot() ?? [],
        workbench: WorkbenchContext(
            clipboardPreview: "https://example.com",
            enabledModuleIDs: gate.snapshot() ?? [],
            pinnedModuleIDs: [.workbenchSnippets]
        )
    )
    #expect(!context.isEnabled(.workbenchSnippets))
    #expect(!context.isHot(.workbenchSnippets))
    #expect(context.isEnabled(.workbenchProjects))
}
