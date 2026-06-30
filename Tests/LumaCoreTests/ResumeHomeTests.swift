import Foundation
import Testing
import LumaCore

@Test func homeEnablementGateFiltersDisabledModules() {
    let gate = HomeEnablementGate()
    let notes = ModuleIdentifier(rawValue: "luma.notes")
    let todo = ModuleIdentifier(rawValue: "luma.todo")
    #expect(gate.contains(notes))
    gate.update([notes])
    #expect(gate.contains(notes))
    #expect(!gate.contains(todo))
}

@Test func resumeHomeAllowsModuleResumeWhenDraftRowsNotShown() {
    #expect(ResumeHomeResumePolicy.allowsModuleResume(
        snippetDraftRowShown: false,
        quicklinkDraftRowShown: false
    ))

    #expect(!ResumeHomeResumePolicy.allowsModuleResume(
        snippetDraftRowShown: true,
        quicklinkDraftRowShown: false
    ))
    #expect(!ResumeHomeResumePolicy.allowsModuleResume(
        snippetDraftRowShown: false,
        quicklinkDraftRowShown: true
    ))
}

@Test func resumeHomeStaleDraftDoesNotBlockModuleResumeWhenSnippetModuleDisabled() {
    let gate = HomeEnablementGate()
    let notes = ModuleIdentifier(rawValue: "luma.notes")
    let snippets = ModuleIdentifier(rawValue: "luma.snippets")
    gate.update([notes])

    let snippetDraftRowShown = gate.contains(snippets)
    #expect(!snippetDraftRowShown)

    #expect(ResumeHomeResumePolicy.allowsModuleResume(
        snippetDraftRowShown: snippetDraftRowShown,
        quicklinkDraftRowShown: false
    ))
}
