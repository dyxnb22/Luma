import Foundation
import LumaCore
import Testing

@Test func launcherModuleResumeQueryFillsEmptyNotesQuery() {
    let notes = ModuleIdentifier(rawValue: "luma.notes")
    let query = LauncherModuleResumeQuery.normalizedQuery(for: notes, raw: "")
    #expect(query == "n ")
}

@Test func launcherModuleResumeQueryPreservesNonEmptyQuery() {
    let notes = ModuleIdentifier(rawValue: "luma.notes")
    let query = LauncherModuleResumeQuery.normalizedQuery(for: notes, raw: "n daily")
    #expect(query == "n daily")
}

@Test func launcherModuleResumeQueryIncludesProjectsAndSecrets() {
    let projects = ModuleIdentifier(rawValue: "luma.projects")
    let secrets = ModuleIdentifier(rawValue: "luma.secrets")
    #expect(LauncherModuleResumeQuery.normalizedQuery(for: projects, raw: "") == "proj ")
    #expect(LauncherModuleResumeQuery.normalizedQuery(for: secrets, raw: "") == "sec ")
    #expect(LauncherModuleResumeQuery.roundTripModules.contains(projects))
    #expect(LauncherModuleResumeQuery.roundTripModules.contains(secrets))
}
