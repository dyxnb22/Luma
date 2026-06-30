import Foundation

public enum WorkbenchActivityKind: String, Sendable, Codable, CaseIterable {
    case opened
    case created
    case converted
    case draftPrepared
    case projectLinked
}

public struct WorkbenchActivityEntry: Sendable, Hashable, Codable, Identifiable {
    public let id: UUID
    public let kind: WorkbenchActivityKind
    public let moduleID: ModuleIdentifier
    public let entityKind: WorkbenchEntityKind?
    public let title: String
    public let detail: String?
    public let recordedAt: Date
    public let entityID: String?
    public let project: WorkbenchProjectAssociation?
    public let sourceAppName: String?
    public let sourceKind: WorkbenchActivitySourceKind?
    public let actionKind: String?
    public let resumeRef: WorkbenchResumeRef?
    public let preview: String?
    public let resumePayloadJSON: Data?

    public init(
        id: UUID = UUID(),
        kind: WorkbenchActivityKind,
        moduleID: ModuleIdentifier,
        entityKind: WorkbenchEntityKind? = nil,
        title: String,
        detail: String? = nil,
        recordedAt: Date = Date(),
        entityID: String? = nil,
        project: WorkbenchProjectAssociation? = nil,
        sourceAppName: String? = nil,
        sourceKind: WorkbenchActivitySourceKind? = nil,
        actionKind: String? = nil,
        resumeRef: WorkbenchResumeRef? = nil,
        preview: String? = nil,
        resumePayloadJSON: Data? = nil
    ) {
        self.id = id
        self.kind = kind
        self.moduleID = moduleID
        self.entityKind = entityKind
        self.title = title
        self.detail = detail
        self.recordedAt = recordedAt
        self.entityID = entityID
        self.project = project
        self.sourceAppName = sourceAppName
        self.sourceKind = sourceKind
        self.actionKind = actionKind
        self.resumeRef = resumeRef
        self.preview = preview
        self.resumePayloadJSON = resumePayloadJSON
    }
}

public struct WorkbenchActivityFile: Codable, Sendable, Equatable {
    public var entries: [WorkbenchActivityEntry]

    public init(entries: [WorkbenchActivityEntry] = []) {
        self.entries = entries
    }
}

private struct WorkbenchActivityEnvelope: Codable, Sendable {
    var version: Int
    var entries: [WorkbenchActivityEntry]
}

/// Local-first activity trail for Continue/Home/Resume.
public actor WorkbenchActivityStore {
    public static let shared = WorkbenchActivityStore()
    public static let schemaVersion = 1

    private let fileURL: URL
    private let maxEntries: Int
    private var entries: [WorkbenchActivityEntry] = []

    public init(
        fileURL: URL = WorkbenchActivityStore.defaultURL(),
        maxEntries: Int = 50
    ) {
        self.fileURL = fileURL
        self.maxEntries = maxEntries
        entries = Self.loadEntries(from: fileURL)
    }

    public static func defaultURL(fileManager: FileManager = .default) -> URL {
        let base = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? fileManager.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support")
        return base.appendingPathComponent("Luma/workbench-activity.json")
    }

    public func snapshot(limit: Int = 10) -> [WorkbenchActivityEntry] {
        WorkbenchActivityQuery.recent(entries: entries, limit: limit)
    }

    public func allEntries() -> [WorkbenchActivityEntry] {
        entries
    }

    public func recent(forProject projectPath: String, limit: Int = 10) -> [WorkbenchActivityEntry] {
        WorkbenchActivityQuery.recent(forProject: projectPath, entries: entries, limit: limit)
    }

    public func recentDrafts(forProject projectPath: String, limit: Int = 5) -> [WorkbenchActivityEntry] {
        WorkbenchActivityQuery.recentDrafts(forProject: projectPath, entries: entries, limit: limit)
    }

    public func recentByModule(_ moduleID: ModuleIdentifier, limit: Int = 10) -> [WorkbenchActivityEntry] {
        WorkbenchActivityQuery.recentByModule(moduleID, entries: entries, limit: limit)
    }

    public func latestProjectContext() -> WorkbenchProjectAssociation? {
        WorkbenchActivityQuery.latestProjectContext(entries: entries)
    }

    public func activitySnapshot(projectIdentity: WorkbenchProjectIdentity?) -> WorkbenchActivitySnapshot {
        WorkbenchActivitySnapshot.build(entries: entries, projectIdentity: projectIdentity)
    }

    public func activitySnapshot(currentProjectPath: String?) -> WorkbenchActivitySnapshot {
        activitySnapshot(
            projectIdentity: currentProjectPath.map {
                WorkbenchProjectIdentity(matchedPath: $0, labelFallback: $0)
            }
        )
    }

    public func record(_ entry: WorkbenchActivityEntry) {
        entries.insert(entry, at: 0)
        if entries.count > maxEntries {
            entries = Array(entries.prefix(maxEntries))
        }
        persist()
    }

    public func recordCapture(
        result: WorkbenchCaptureResult,
        context: WorkbenchContext,
        attribution: WorkbenchCaptureAttribution
    ) {
        let entryID = UUID()
        let previewText = String(result.preview.prefix(80))
        let kind: WorkbenchActivityKind = result.target == .projectSnippetDraft ? .projectLinked : .draftPrepared
        let project = context.currentProject.map(WorkbenchProjectAssociation.init(context:))
        let payload = WorkbenchActivityResumePayload.from(result: result)
        record(WorkbenchActivityEntry(
            id: entryID,
            kind: kind,
            moduleID: result.moduleID,
            entityKind: entityKind(for: result.target),
            title: title(for: result.target, project: project),
            detail: previewText,
            entityID: nil,
            project: project,
            sourceAppName: context.frontmostAppName,
            sourceKind: attribution.sourceKind,
            actionKind: attribution.followUp.rawValue,
            resumeRef: resumeRef(for: result, entryID: entryID, payload: payload),
            preview: previewText,
            resumePayloadJSON: payload?.encoded()
        ))
    }

    public func recordCapture(
        target: WorkbenchCaptureTarget,
        moduleID: ModuleIdentifier,
        preview: String
    ) {
        record(WorkbenchActivityEntry(
            kind: .draftPrepared,
            moduleID: moduleID,
            entityKind: entityKind(for: target),
            title: "Prepared \(target.displayName)",
            detail: String(preview.prefix(80)),
            preview: String(preview.prefix(80))
        ))
    }

    private func title(
        for target: WorkbenchCaptureTarget,
        project: WorkbenchProjectAssociation?
    ) -> String {
        if let project, target == .projectSnippetDraft {
            return "Project snippet for \(project.projectLabel)"
        }
        return "Prepared \(target.displayName)"
    }

    private func resumeRef(
        for result: WorkbenchCaptureResult,
        entryID: UUID,
        payload: WorkbenchActivityResumePayload?
    ) -> WorkbenchResumeRef? {
        guard payload != nil else { return nil }
        switch result.target {
        case .snippetDraft, .projectSnippetDraft:
            return WorkbenchResumeRef(kind: .snippetDraft, entryID: entryID)
        case .quicklinkDraft:
            return WorkbenchResumeRef(kind: .quicklinkDraft, entryID: entryID)
        case .todoDraft:
            return WorkbenchResumeRef(kind: .todoCapture, entryID: entryID)
        case .noteDraft:
            return nil
        }
    }

    private func entityKind(for target: WorkbenchCaptureTarget) -> WorkbenchEntityKind {
        switch target {
        case .noteDraft: .note
        case .todoDraft: .todo
        case .snippetDraft: .snippet
        case .quicklinkDraft: .quicklink
        case .projectSnippetDraft: .snippet
        }
    }

    private static func loadEntries(from fileURL: URL) -> [WorkbenchActivityEntry] {
        guard let data = try? Data(contentsOf: fileURL) else { return [] }
        let decoder = JSONDecoder()
        if let envelope = try? decoder.decode(WorkbenchActivityEnvelope.self, from: data),
           envelope.version == schemaVersion {
            return envelope.entries
        }
        if let legacy = try? decoder.decode(WorkbenchActivityFile.self, from: data) {
            return legacy.entries
        }
        return []
    }

    private func persist() {
        let dir = fileURL.deletingLastPathComponent()
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let envelope = WorkbenchActivityEnvelope(version: Self.schemaVersion, entries: entries)
        guard let data = try? JSONEncoder().encode(envelope) else { return }
        try? data.write(to: fileURL, options: .atomic)
    }
}
