import Foundation

/// Unified read model for workbench activity across commands and project detail surfaces.
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
        projectIdentity: ProjectIdentity?,
        globalLimit: Int = globalRecentLimit,
        projectRecentLimit: Int = projectRecentLimit,
        projectDraftsLimit: Int = projectDraftsLimit
    ) -> WorkbenchActivitySnapshot {
        let global = WorkbenchActivityQuery.recent(entries: entries, limit: globalLimit)
        guard let identity = projectIdentity else {
            return WorkbenchActivitySnapshot(globalRecent: global)
        }
        let projectRecent = WorkbenchActivityQuery.recent(
            for: identity,
            entries: entries,
            limit: projectRecentLimit
        )
        let projectDrafts = WorkbenchActivityQuery.recentDrafts(
            for: identity,
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
        projectIdentity: WorkbenchProjectIdentity?,
        globalLimit: Int = globalRecentLimit,
        projectRecentLimit: Int = projectRecentLimit,
        projectDraftsLimit: Int = projectDraftsLimit
    ) -> WorkbenchActivitySnapshot {
        build(
            entries: entries,
            projectIdentity: projectIdentity?.identity,
            globalLimit: globalLimit,
            projectRecentLimit: projectRecentLimit,
            projectDraftsLimit: projectDraftsLimit
        )
    }

    @available(*, deprecated, message: "Use build(entries:projectIdentity:)")
    public static func build(
        entries: [WorkbenchActivityEntry],
        currentProjectPath: String?,
        globalLimit: Int = globalRecentLimit,
        projectRecentLimit: Int = projectRecentLimit,
        projectDraftsLimit: Int = projectDraftsLimit
    ) -> WorkbenchActivitySnapshot {
        let identity: ProjectIdentity?
        if let path = currentProjectPath?.trimmingCharacters(in: .whitespacesAndNewlines), !path.isEmpty {
            if ProjectIdentity.looksLikePath(path) {
                identity = ProjectIdentity(
                    stableProjectID: ProjectIdentity.makeStableID(
                        matchedPath: path,
                        labelFallback: path,
                        sourceBundleID: nil
                    ),
                    matchedPath: path,
                    labelFallback: path
                )
            } else {
                identity = ProjectIdentity(
                    stableProjectID: ProjectIdentity.makeStableID(
                        matchedPath: nil,
                        labelFallback: path,
                        sourceBundleID: nil
                    ),
                    matchedPath: nil,
                    labelFallback: path
                )
            }
        } else {
            identity = nil
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
