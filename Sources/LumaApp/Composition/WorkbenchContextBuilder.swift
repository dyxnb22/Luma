import Foundation
import LumaCore
import LumaModules
import LumaServices

/// Assembles a `WorkbenchContext` snapshot from cached launcher signals.
struct WorkbenchContextBuilder {
    func build(
        enabledModuleIDs: Set<ModuleIdentifier>,
        pinnedModuleIDs: Set<ModuleIdentifier>,
        clipboardPreview: String?,
        selectionText: String?
    ) async -> WorkbenchContext {
        let project = await CurrentProjectService.shared.snapshot()
        let resume = LauncherResumeStore.load()
        var pendingDrafts: [WorkbenchDraftRef] = []

        if let data = resume.snippetDraftJSON,
           let draft = try? JSONDecoder().decode(SnippetDraft.self, from: data) {
            pendingDrafts.append(WorkbenchDraftRef(
                target: .snippetDraft,
                moduleID: .snippets,
                preview: draft.trigger
            ))
        }
        if let data = resume.quicklinkDraftJSON,
           let draft = try? JSONDecoder().decode(URLQuicklinkDraft.self, from: data) {
            pendingDrafts.append(WorkbenchDraftRef(
                target: .quicklinkDraft,
                moduleID: .quicklinks,
                preview: draft.trigger
            ))
        }
        if let todoText = resume.todoCaptureText, !todoText.isEmpty {
            pendingDrafts.append(WorkbenchDraftRef(
                target: .todoDraft,
                moduleID: .todo,
                preview: String(todoText.prefix(48))
            ))
        }

        let clipboardURL = clipboardPreview.flatMap { URLTextParser.firstHTTPURL(in: $0) }
        let projectIdentity = project.map(WorkbenchProjectIdentity.init(context:))
        async let activitySnapshot = WorkbenchActivityStore.shared.activitySnapshot(
            projectIdentity: projectIdentity
        )
        async let linkSnapshot = WorkbenchLinkStore.shared.snapshot(
            for: projectIdentity?.identity,
            limit: 10
        )
        let activity = await activitySnapshot
        let links = await linkSnapshot

        return WorkbenchContext(
            selectionText: selectionText,
            clipboardPreview: clipboardPreview,
            clipboardURL: clipboardURL,
            frontmostAppName: project?.frontAppName,
            currentProject: project,
            pendingDrafts: pendingDrafts,
            enabledModuleIDs: enabledModuleIDs,
            pinnedModuleIDs: pinnedModuleIDs,
            activitySnapshot: activity,
            linkSnapshot: WorkbenchLinkSnapshot(currentProjectLinks: links)
        )
    }
}
