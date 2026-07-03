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

@Test func launcherModuleResumeQueryTitlesAreModuleSpecific() {
    #expect(LauncherModuleResumeQuery.resumeTitle(for: ModuleIdentifier(rawValue: "luma.notes")) == "Resume Notes search")
    #expect(LauncherModuleResumeQuery.resumeTitle(for: ModuleIdentifier(rawValue: "luma.wordbook")) == "Resume Wordbook")
    #expect(LauncherModuleResumeQuery.resumeTitle(for: ModuleIdentifier(rawValue: "luma.media")) == "Resume Records")
    #expect(LauncherModuleResumeQuery.resumeTitle(for: ModuleIdentifier(rawValue: "luma.projects")) == "Resume Projects search")
    #expect(LauncherModuleResumeQuery.resumeTitle(for: ModuleIdentifier(rawValue: "luma.secrets")) == "Resume Secrets search")
}

@Test func launcherModuleResumeQueryIncludesProjectsAndSecrets() {
    let projects = ModuleIdentifier(rawValue: "luma.projects")
    let secrets = ModuleIdentifier(rawValue: "luma.secrets")
    #expect(LauncherModuleResumeQuery.normalizedQuery(for: projects, raw: "") == "proj ")
    #expect(LauncherModuleResumeQuery.normalizedQuery(for: secrets, raw: "") == "sec ")
    #expect(LauncherModuleResumeQuery.roundTripModules.contains(projects))
    #expect(LauncherModuleResumeQuery.roundTripModules.contains(secrets))
}
