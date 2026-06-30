import Foundation

/// Pure query helpers over in-memory activity entries.
public enum WorkbenchActivityQuery {
    public static func recent(entries: [WorkbenchActivityEntry], limit: Int = 10) -> [WorkbenchActivityEntry] {
        Array(entries.prefix(max(0, limit)))
    }

    public static func recent(
        forProject projectPath: String,
        entries: [WorkbenchActivityEntry],
        limit: Int = 10
    ) -> [WorkbenchActivityEntry] {
        let normalized = projectPath.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !normalized.isEmpty else { return [] }
        return entries
            .filter { $0.project?.projectPath == normalized }
            .prefix(max(0, limit))
            .map { $0 }
    }

    public static func recentDrafts(
        forProject projectPath: String,
        entries: [WorkbenchActivityEntry],
        limit: Int = 5
    ) -> [WorkbenchActivityEntry] {
        recent(forProject: projectPath, entries: entries, limit: entries.count)
            .filter { $0.isResumableDraft && ($0.kind == .draftPrepared || $0.kind == .projectLinked) }
            .prefix(max(0, limit))
            .map { $0 }
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
        entries.first { $0.project != nil }?.project
    }
}
