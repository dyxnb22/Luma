import Foundation

/// Unified read model for workbench activity across Home, commands, and project detail.
public struct WorkbenchActivitySnapshot: Sendable, Equatable {
    public static let globalRecentLimit = 8
    public static let projectRecentLimit = 5
    public static let projectDraftsLimit = 5

    public let globalRecent: [WorkbenchActivityEntry]
    public let currentProjectRecent: [WorkbenchActivityEntry]
    public let currentProjectDrafts: [WorkbenchActivityEntry]

    public init(
        globalRecent: [WorkbenchActivityEntry] = [],
        currentProjectRecent: [WorkbenchActivityEntry] = [],
        currentProjectDrafts: [WorkbenchActivityEntry] = []
    ) {
        self.globalRecent = globalRecent
        self.currentProjectRecent = currentProjectRecent
        self.currentProjectDrafts = currentProjectDrafts
    }

    public static func build(
        entries: [WorkbenchActivityEntry],
        projectIdentity: WorkbenchProjectIdentity?,
        globalLimit: Int = globalRecentLimit,
        projectRecentLimit: Int = projectRecentLimit,
        projectDraftsLimit: Int = projectDraftsLimit
    ) -> WorkbenchActivitySnapshot {
        let global = WorkbenchActivityQuery.recent(entries: entries, limit: globalLimit)
        guard let identity = projectIdentity,
              let queryKey = identity.activityQueryKey else {
            return WorkbenchActivitySnapshot(globalRecent: global)
        }
        let projectRecent = WorkbenchActivityQuery.recent(
            forProject: queryKey,
            entries: entries,
            limit: projectRecentLimit
        )
        let projectDrafts = WorkbenchActivityQuery.recentDrafts(
            forProject: queryKey,
            entries: entries,
            limit: projectDraftsLimit
        )
        return WorkbenchActivitySnapshot(
            globalRecent: global,
            currentProjectRecent: projectRecent,
            currentProjectDrafts: projectDrafts
        )
    }

    public static func build(
        entries: [WorkbenchActivityEntry],
        currentProjectPath: String?,
        globalLimit: Int = globalRecentLimit,
        projectRecentLimit: Int = projectRecentLimit,
        projectDraftsLimit: Int = projectDraftsLimit
    ) -> WorkbenchActivitySnapshot {
        let identity = currentProjectPath.map {
            WorkbenchProjectIdentity(matchedPath: $0, labelFallback: $0)
        }
        return build(
            entries: entries,
            projectIdentity: identity,
            globalLimit: globalLimit,
            projectRecentLimit: projectRecentLimit,
            projectDraftsLimit: projectDraftsLimit
        )
    }

    public func enabledCurrentProjectRecent(
        enabledModuleIDs: Set<ModuleIdentifier>,
        limit: Int = 3,
        excludingResumableDrafts: Bool = false
    ) -> [WorkbenchActivityEntry] {
        currentProjectRecent
            .filter { enabledModuleIDs.contains($0.moduleID) }
            .filter { !excludingResumableDrafts || !$0.isResumableDraft }
            .prefix(max(0, limit))
            .map { $0 }
    }

    public func enabledCurrentProjectDrafts(
        enabledModuleIDs: Set<ModuleIdentifier>,
        limit: Int = 1
    ) -> [WorkbenchActivityEntry] {
        currentProjectDrafts
            .filter { $0.isResumableDraft && enabledModuleIDs.contains($0.moduleID) }
            .prefix(max(0, limit))
            .map { $0 }
    }
}
