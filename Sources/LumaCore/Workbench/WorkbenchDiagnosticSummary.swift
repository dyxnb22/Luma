import Foundation

/// Read-only workbench diagnostic text assembled from a context snapshot.
public struct WorkbenchDiagnosticSummary: Sendable, Equatable {
    public let stableProjectID: String?
    public let projectLabel: String?
    public let projectActivityCount: Int
    public let projectLinkCount: Int
    public let enabledProjectLinkCount: Int
    public let subtitle: String
    public let fullMessage: String

    public static func from(context: WorkbenchContext) -> WorkbenchDiagnosticSummary {
        let identity = context.currentProject.map(WorkbenchProjectIdentity.init(context:))
        let stableProjectID = identity?.stableProjectID
        let projectLabel = context.currentProject?.projectName ?? context.currentProject?.projectLabel

        let counts = context.projectIndexCounts ?? WorkbenchProjectIndexCounts(
            projectActivityCount: context.activitySnapshot.currentProjectRecent.count,
            projectLinkCount: context.linkSnapshot.currentProjectLinks.count,
            enabledProjectLinkCount: context.linkSnapshot.enabledLinks(
                enabledModuleIDs: context.enabledModuleIDs,
                limit: Int.max
            ).count
        )

        let projectIDText = stableProjectID.map(shortenStableID) ?? "none"
        let subtitle = "Project: \(projectLabel ?? "none") · ID: \(projectIDText) · \(counts.projectActivityCount) activities · \(counts.projectLinkCount) indexed links (\(counts.enabledProjectLinkCount) enabled)"
        let fullMessage = """
        Workbench status
        Project: \(projectLabel ?? "none")
        stableProjectID: \(stableProjectID ?? "none")
        Project activities in store: \(counts.projectActivityCount)
        Indexed links for project: \(counts.projectLinkCount)
        Enabled-module links: \(counts.enabledProjectLinkCount)
        """

        return WorkbenchDiagnosticSummary(
            stableProjectID: stableProjectID,
            projectLabel: projectLabel,
            projectActivityCount: counts.projectActivityCount,
            projectLinkCount: counts.projectLinkCount,
            enabledProjectLinkCount: counts.enabledProjectLinkCount,
            subtitle: subtitle,
            fullMessage: fullMessage
        )
    }

    private static func shortenStableID(_ id: String) -> String {
        if id.count <= 24 { return id }
        return String(id.prefix(12)) + "…" + String(id.suffix(6))
    }
}

/// Computes full project index counts from in-memory store entries (read-only).
public enum WorkbenchProjectIndexCountsBuilder {
    public static func build(
        projectIdentity: WorkbenchProjectIdentity?,
        entries: [WorkbenchActivityEntry],
        projectLinks: [WorkbenchProjectLink],
        enabledModuleIDs: Set<ModuleIdentifier>
    ) -> WorkbenchProjectIndexCounts? {
        guard let projectIdentity else { return nil }
        let activityCount = WorkbenchActivityQuery.recent(
            for: projectIdentity.identity,
            entries: entries,
            limit: entries.count
        ).count
        let enabledLinkCount = projectLinks
            .filter { enabledModuleIDs.contains($0.entityRef.moduleID) }
            .count
        return WorkbenchProjectIndexCounts(
            projectActivityCount: activityCount,
            projectLinkCount: projectLinks.count,
            enabledProjectLinkCount: enabledLinkCount
        )
    }
}
