import Foundation

/// Pure query helpers over in-memory activity entries.
public enum WorkbenchActivityQuery {
    public static func recent(entries: [WorkbenchActivityEntry], limit: Int = 10) -> [WorkbenchActivityEntry] {
        Array(entries.prefix(max(0, limit)))
    }

    public static func recent(
        for identity: ProjectIdentity,
        entries: [WorkbenchActivityEntry],
        limit: Int = 10
    ) -> [WorkbenchActivityEntry] {
        entries
            .filter { matchesProject($0, identity: identity) }
            .prefix(max(0, limit))
            .map { $0 }
    }

    public static func recent(
        forProject projectPath: String,
        entries: [WorkbenchActivityEntry],
        limit: Int = 10
    ) -> [WorkbenchActivityEntry] {
        let normalized = projectPath.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !normalized.isEmpty else { return [] }
        if ProjectIdentity.looksLikePath(normalized) {
            let identity = ProjectIdentity(
                stableProjectID: ProjectIdentity.makeStableID(
                    matchedPath: normalized,
                    labelFallback: normalized,
                    sourceBundleID: nil
                ),
                matchedPath: normalized,
                labelFallback: normalized
            )
            return recent(for: identity, entries: entries, limit: limit)
        }
        let identity = ProjectIdentity(
            stableProjectID: ProjectIdentity.makeStableID(
                matchedPath: nil,
                labelFallback: normalized,
                sourceBundleID: nil
            ),
            matchedPath: nil,
            labelFallback: normalized
        )
        return recent(for: identity, entries: entries, limit: limit)
    }

    public static func recentDrafts(
        for identity: ProjectIdentity,
        entries: [WorkbenchActivityEntry],
        limit: Int = 5
    ) -> [WorkbenchActivityEntry] {
        recent(for: identity, entries: entries, limit: entries.count)
            .filter { $0.isResumableDraft && ($0.kind == .draftPrepared || $0.kind == .projectLinked) }
            .prefix(max(0, limit))
            .map { $0 }
    }

    public static func recentDrafts(
        forProject projectPath: String,
        entries: [WorkbenchActivityEntry],
        limit: Int = 5
    ) -> [WorkbenchActivityEntry] {
        let normalized = projectPath.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !normalized.isEmpty else { return [] }
        let identity: ProjectIdentity
        if ProjectIdentity.looksLikePath(normalized) {
            identity = ProjectIdentity(
                stableProjectID: ProjectIdentity.makeStableID(
                    matchedPath: normalized,
                    labelFallback: normalized,
                    sourceBundleID: nil
                ),
                matchedPath: normalized,
                labelFallback: normalized
            )
        } else {
            identity = ProjectIdentity(
                stableProjectID: ProjectIdentity.makeStableID(
                    matchedPath: nil,
                    labelFallback: normalized,
                    sourceBundleID: nil
                ),
                matchedPath: nil,
                labelFallback: normalized
            )
        }
        return recentDrafts(for: identity, entries: entries, limit: limit)
    }

    public static func recentByModule(
        _ moduleID: ModuleIdentifier,
        entries: [WorkbenchActivityEntry],
        limit: Int = 10
    ) -> [WorkbenchActivityEntry] {
        entries
            .filter { $0.moduleID == moduleID }
            .prefix(max(0, limit))
            .map { $0 }
    }

    public static func latestProjectContext(entries: [WorkbenchActivityEntry]) -> WorkbenchProjectAssociation? {
        entries.first { $0.project != nil || $0.projectIdentity != nil }?.legacyProjectAssociation
    }

    private static func matchesProject(_ entry: WorkbenchActivityEntry, identity: ProjectIdentity) -> Bool {
        if let entryIdentity = entry.projectIdentity {
            if entryIdentity.stableProjectID == identity.stableProjectID {
                return true
            }
            if entryIdentity.matchedPath == nil,
               identity.matchedPath == nil,
               ProjectIdentity.isLegacyLabelStableID(entryIdentity.stableProjectID),
               entryIdentity.labelFallback == identity.labelFallback {
                return true
            }
            return false
        }
        guard let project = entry.project else { return false }
        if let queryKey = identity.activityQueryKey, project.projectPath == queryKey {
            return true
        }
        if let matchedPath = identity.matchedPath, project.projectPath == matchedPath {
            return true
        }
        if identity.matchedPath == nil, project.projectPath == identity.labelFallback {
            return true
        }
        return false
    }
}

private extension WorkbenchActivityEntry {
    var legacyProjectAssociation: WorkbenchProjectAssociation? {
        if let project { return project }
        guard let identity = projectIdentity else { return nil }
        return WorkbenchProjectAssociation(
            projectPath: identity.matchedPath ?? identity.labelFallback,
            projectLabel: identity.labelFallback,
            projectName: identity.displayName
        )
    }
}
