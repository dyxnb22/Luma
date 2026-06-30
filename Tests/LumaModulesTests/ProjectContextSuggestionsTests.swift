import Foundation
import LumaCore
import LumaModules
import Testing

@Test func projectContextSnippetDraftIncludesProjectTags() {
    let context = CurrentProjectContext(
        frontAppName: "Code",
        bundleID: "com.microsoft.VSCode",
        windowTitle: "App.swift — Luma",
        projectLabel: "Luma",
        filename: "App.swift",
        matchedProjectPath: "/Users/me/Luma",
        matchedProjectName: "Luma"
    )
    let draft = ProjectContextSuggestions.snippetDraft(for: context)
    #expect(draft.title == "Luma snippet")
    #expect(draft.trigger == ";luma")
    #expect(draft.tags.contains("luma"))
    #expect(draft.tags.contains("project"))
    #expect(draft.content.contains("{{project}}"))
}

@Test func projectContextQuicklinkDraftUsesFolderURL() {
    let context = CurrentProjectContext(
        frontAppName: "Code",
        bundleID: "com.microsoft.VSCode",
        windowTitle: "readme.md",
        projectLabel: "Luma",
        filename: "readme.md",
        matchedProjectPath: "/Users/me/Luma",
        matchedProjectName: "Luma"
    )
    let draft = ProjectContextSuggestions.quicklinkDraft(for: context)
    #expect(draft?.name == "Luma folder")
    #expect(draft?.trigger == "luma")
    #expect(draft?.urlTemplate == URL(fileURLWithPath: "/Users/me/Luma").absoluteString)
}

@Test func projectQuicklinkDraftSourceMatchesContextSuggestionFacade() {
    let context = CurrentProjectContext(
        frontAppName: "Code",
        bundleID: "com.microsoft.VSCode",
        windowTitle: "readme.md",
        projectLabel: "Luma",
        filename: "readme.md",
        matchedProjectPath: "/Users/me/Luma",
        matchedProjectName: "Luma"
    )
    #expect(ProjectQuicklinkDraftSource(context: context).quicklinkDraft() == ProjectContextSuggestions.quicklinkDraft(for: context))
}

@Test func projectContextQuicklinkDraftNilWithoutPath() {
    let context = CurrentProjectContext(
        frontAppName: "Safari",
        bundleID: "com.apple.Safari",
        windowTitle: "Example",
        projectLabel: "Example",
        filename: nil
    )
    #expect(ProjectContextSuggestions.quicklinkDraft(for: context) == nil)
}
