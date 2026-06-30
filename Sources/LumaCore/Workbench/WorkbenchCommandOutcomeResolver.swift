import Foundation

/// Counts for the current project index used in diagnostics (full store scan, not UI limits).
public struct WorkbenchProjectIndexCounts: Sendable, Equatable {
    public let projectActivityCount: Int
    public let projectLinkCount: Int
    public let enabledProjectLinkCount: Int

    public init(
        projectActivityCount: Int,
        projectLinkCount: Int,
        enabledProjectLinkCount: Int
    ) {
        self.projectActivityCount = projectActivityCount
        self.projectLinkCount = projectLinkCount
        self.enabledProjectLinkCount = enabledProjectLinkCount
    }
}

/// Resolves bare workbench command outcomes from a built context (no module fan-out).
public enum WorkbenchCommandOutcomeResolver {
    public static func projectRecent(context: WorkbenchContext) -> WorkbenchCommandOutcome {
        guard context.isEnabled(.workbenchProjects), context.currentProject != nil else {
            return .status(WorkbenchEmptyStateCopy.noProjectContext)
        }
        let recent = context.activitySnapshot.enabledCurrentProjectRecent(
            enabledModuleIDs: context.enabledModuleIDs,
            limit: 1
        )
        guard let top = recent.first else {
            return .status(WorkbenchEmptyStateCopy.noRecentActivity)
        }
        let rowAction = WorkbenchLinkedEntityOpenPlanner.rowAction(for: top)
        if let outcome = WorkbenchWorkspaceRowActionCodec.commandOutcome(for: rowAction, entry: top) {
            return outcome
        }
        return .status(WorkbenchEmptyStateCopy.noRecentActivity)
    }

    public static func projectLinks(context: WorkbenchContext) -> WorkbenchCommandOutcome {
        guard context.isEnabled(.workbenchProjects), context.currentProject != nil else {
            return .status(WorkbenchEmptyStateCopy.noProjectContext)
        }
        let links = context.linkSnapshot.enabledLinks(
            enabledModuleIDs: context.enabledModuleIDs,
            limit: 1
        )
        guard let link = links.first else {
            return .status(WorkbenchEmptyStateCopy.noLinkedItems)
        }
        return .openLinked(link.id)
    }
}
