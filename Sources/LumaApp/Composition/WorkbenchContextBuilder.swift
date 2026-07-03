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

        let clipboardURL = clipboardPreview.flatMap { URLTextParser.firstHTTPURL(in: $0) }
        let projectIdentity = project.map(WorkbenchProjectIdentity.init(context:))
        async let allEntries = WorkbenchActivityStore.shared.allEntries()
        async let activitySnapshot = WorkbenchActivityStore.shared.activitySnapshot(
            projectIdentity: projectIdentity
        )
        let entries = await allEntries
        await WorkbenchLinkStore.shared.ensureLinksIndexed(
            for: projectIdentity?.identity,
            from: entries
        )
        async let linkSnapshot = WorkbenchLinkStore.shared.snapshot(
            for: projectIdentity?.identity,
            limit: 10
        )
        async let allProjectLinks = WorkbenchLinkStore.shared.snapshot(
            for: projectIdentity?.identity,
            limit: 100
        )
        let activity = await activitySnapshot
        let links = await linkSnapshot
        let allLinks = await allProjectLinks
        let indexCounts = WorkbenchProjectIndexCountsBuilder.build(
            projectIdentity: projectIdentity,
            entries: entries,
            projectLinks: allLinks,
            enabledModuleIDs: enabledModuleIDs
        )

        return WorkbenchContext(
            selectionText: selectionText,
            clipboardPreview: clipboardPreview,
            clipboardURL: clipboardURL,
            frontmostAppName: project?.frontAppName,
            currentProject: project,
            enabledModuleIDs: enabledModuleIDs,
            pinnedModuleIDs: pinnedModuleIDs,
            activitySnapshot: activity,
            linkSnapshot: WorkbenchLinkSnapshot(currentProjectLinks: links),
            projectIndexCounts: indexCounts
        )
    }
}
