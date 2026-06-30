import Foundation
import Testing
import LumaCore

@Test func homeContributionContextIsHotRequiresPinAndEnable() {
    let notes = ModuleIdentifier(rawValue: "luma.notes")
    let todo = ModuleIdentifier(rawValue: "luma.todo")
    let context = HomeContributionContext(
        pinnedModuleIDs: [notes],
        enabledModuleIDs: [notes, todo]
    )
    #expect(context.isEnabled(notes))
    #expect(context.isHot(notes))
    #expect(context.isEnabled(todo))
    #expect(!context.isHot(todo))
}

@Test func launcherModuleResumeQueryIncludesProjectsAndSecrets() {
    let projects = ModuleIdentifier(rawValue: "luma.projects")
    let secrets = ModuleIdentifier(rawValue: "luma.secrets")
    #expect(LauncherModuleResumeQuery.normalizedQuery(for: projects, raw: "") == "proj ")
    #expect(LauncherModuleResumeQuery.normalizedQuery(for: secrets, raw: "") == "sec ")
    #expect(LauncherModuleResumeQuery.roundTripModules.contains(projects))
    #expect(LauncherModuleResumeQuery.roundTripModules.contains(secrets))
}
