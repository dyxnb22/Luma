import Foundation

public enum WorkbenchActivityKind: String, Sendable, Codable, CaseIterable {
    case opened
    case created
    case converted
    case draftPrepared
    case projectLinked
}

public struct WorkbenchActivityEntry: Sendable, Hashable, Identifiable, Codable {
    public let id: UUID
    public let kind: WorkbenchActivityKind
    public let moduleID: ModuleIdentifier
    public let entityKind: WorkbenchEntityKind?
    public let title: String
    public let detail: String?
    public let recordedAt: Date
    public let entityID: String?
    public let projectIdentity: ProjectIdentity?
    public let entityRef: WorkbenchEntityRef?
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
        projectIdentity: ProjectIdentity? = nil,
        entityRef: WorkbenchEntityRef? = nil,
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
        self.projectIdentity = projectIdentity
        self.entityRef = entityRef
        self.project = project
        self.sourceAppName = sourceAppName
        self.sourceKind = sourceKind
        self.actionKind = actionKind
        self.resumeRef = resumeRef
        self.preview = preview
        self.resumePayloadJSON = resumePayloadJSON
    }

    enum CodingKeys: String, CodingKey {
        case id, kind, moduleID, entityKind, title, detail, recordedAt, entityID
        case projectIdentity, entityRef, project
        case sourceAppName, sourceKind, actionKind, resumeRef, preview, resumePayloadJSON
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(UUID.self, forKey: .id)
        kind = try container.decode(WorkbenchActivityKind.self, forKey: .kind)
        moduleID = try container.decode(ModuleIdentifier.self, forKey: .moduleID)
        entityKind = try container.decodeIfPresent(WorkbenchEntityKind.self, forKey: .entityKind)
        title = try container.decode(String.self, forKey: .title)
        detail = try container.decodeIfPresent(String.self, forKey: .detail)
        recordedAt = try container.decodeIfPresent(Date.self, forKey: .recordedAt) ?? Date()
        entityID = try container.decodeIfPresent(String.self, forKey: .entityID)
        projectIdentity = try container.decodeIfPresent(ProjectIdentity.self, forKey: .projectIdentity)
        entityRef = try container.decodeIfPresent(WorkbenchEntityRef.self, forKey: .entityRef)
        project = try container.decodeIfPresent(WorkbenchProjectAssociation.self, forKey: .project)
        sourceAppName = try container.decodeIfPresent(String.self, forKey: .sourceAppName)
        sourceKind = try container.decodeIfPresent(WorkbenchActivitySourceKind.self, forKey: .sourceKind)
        actionKind = try container.decodeIfPresent(String.self, forKey: .actionKind)
        resumeRef = try container.decodeIfPresent(WorkbenchResumeRef.self, forKey: .resumeRef)
        preview = try container.decodeIfPresent(String.self, forKey: .preview)
        resumePayloadJSON = try container.decodeIfPresent(Data.self, forKey: .resumePayloadJSON)
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(id, forKey: .id)
        try container.encode(kind, forKey: .kind)
        try container.encode(moduleID, forKey: .moduleID)
        try container.encodeIfPresent(entityKind, forKey: .entityKind)
        try container.encode(title, forKey: .title)
        try container.encodeIfPresent(detail, forKey: .detail)
        try container.encode(recordedAt, forKey: .recordedAt)
        try container.encodeIfPresent(entityID, forKey: .entityID)
        try container.encodeIfPresent(projectIdentity, forKey: .projectIdentity)
        try container.encodeIfPresent(entityRef, forKey: .entityRef)
        try container.encodeIfPresent(sourceAppName, forKey: .sourceAppName)
        try container.encodeIfPresent(sourceKind, forKey: .sourceKind)
        try container.encodeIfPresent(actionKind, forKey: .actionKind)
        try container.encodeIfPresent(resumeRef, forKey: .resumeRef)
        try container.encodeIfPresent(preview, forKey: .preview)
        try container.encodeIfPresent(resumePayloadJSON, forKey: .resumePayloadJSON)
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

/// Local-first activity trail for workbench resume and project flows.
public actor WorkbenchActivityStore {
    public static let shared = WorkbenchActivityStore()
    public static let schemaVersion = 2
    private static let legacySchemaVersion = 1

    private let fileURL: URL
    private let maxEntries: Int
    private var entries: [WorkbenchActivityEntry] = []

    public init(
        fileURL: URL = WorkbenchActivityStore.defaultURL(),
        maxEntries: Int = 50
    ) {
        self.fileURL = fileURL
        self.maxEntries = maxEntries
        let loaded = Self.loadEntries(from: fileURL)
        entries = loaded.entries
        if loaded.migrated {
            Self.persistEntries(entries, to: fileURL)
        }
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
        WorkbenchActivitySnapshot.build(
            entries: entries,
            projectIdentity: projectIdentity?.identity
        )
    }

    @available(*, deprecated, message: "Use activitySnapshot(projectIdentity:)")
    public func activitySnapshot(currentProjectPath: String?) -> WorkbenchActivitySnapshot {
        activitySnapshot(
            projectIdentity: currentProjectPath.map {
                WorkbenchProjectIdentity(matchedPath: $0, labelFallback: $0)
            }
        )
    }

    public func record(_ entry: WorkbenchActivityEntry) {
        entries.insert(Self.normalizeForPersist(entry), at: 0)
        if entries.count > maxEntries {
            entries = Array(entries.prefix(maxEntries))
        }
        persistEntries()
    }

    public func recordCapture(
        result: WorkbenchCaptureResult,
        context: WorkbenchContext,
        attribution: WorkbenchCaptureAttribution
    ) -> WorkbenchActivityEntry {
        let entryID = UUID()
        let previewText = String(result.preview.prefix(80))
        let kind: WorkbenchActivityKind = result.target == .projectSnippetDraft ? .projectLinked : .draftPrepared
        let projectIdentity = context.currentProject.map(ProjectIdentity.init(context:))
        let entityKindValue = entityKind(for: result.target)
        let entryTitle = title(for: result.target, projectIdentity: projectIdentity)
        let payload = WorkbenchActivityResumePayload.from(result: result)
        let entityRef = WorkbenchEntityRef(
            kind: entityKindValue,
            entityID: entryID.uuidString,
            moduleID: result.moduleID,
            title: entryTitle,
            subtitle: previewText
        )
        let entry = WorkbenchActivityEntry(
            id: entryID,
            kind: kind,
            moduleID: result.moduleID,
            entityKind: entityKindValue,
            title: entryTitle,
            detail: previewText,
            entityID: entryID.uuidString,
            projectIdentity: projectIdentity,
            entityRef: entityRef,
            sourceAppName: context.frontmostAppName,
            sourceKind: attribution.sourceKind,
            actionKind: attribution.followUp.rawValue,
            resumeRef: resumeRef(for: result, entryID: entryID, payload: payload),
            preview: previewText,
            resumePayloadJSON: payload?.encoded()
        )
        record(entry)
        return entry
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
        projectIdentity: ProjectIdentity?
    ) -> String {
        if let projectIdentity, target == .projectSnippetDraft {
            let name = projectIdentity.displayName ?? projectIdentity.labelFallback
            return "Project snippet for \(name)"
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
            return WorkbenchResumeRef(kind: .noteAction, entryID: entryID)
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

    private static func loadEntries(from fileURL: URL) -> (entries: [WorkbenchActivityEntry], migrated: Bool) {
        guard let data = try? Data(contentsOf: fileURL) else { return ([], false) }
        let decoder = JSONDecoder()
        if let envelope = try? decoder.decode(WorkbenchActivityEnvelope.self, from: data) {
            if envelope.version == schemaVersion {
                return (envelope.entries.map(normalizeForPersist), false)
            }
            if envelope.version == legacySchemaVersion {
                let migrated = migrateEntries(envelope.entries)
                return (migrated, true)
            }
        }
        if let legacy = try? decoder.decode(WorkbenchActivityFile.self, from: data) {
            let migrated = migrateEntries(legacy.entries)
            return (migrated, true)
        }
        return ([], false)
    }

    public static func migrateEntries(_ entries: [WorkbenchActivityEntry]) -> [WorkbenchActivityEntry] {
        entries.map(migrateEntry)
    }

    public static func migrateEntry(_ entry: WorkbenchActivityEntry) -> WorkbenchActivityEntry {
        let identity: ProjectIdentity?
        if let existing = entry.projectIdentity {
            identity = existing
        } else if let project = entry.project {
            identity = ProjectIdentity(legacy: project, sourceBundleID: nil)
        } else {
            identity = nil
        }

        var migrated = WorkbenchActivityEntry(
            id: entry.id,
            kind: entry.kind,
            moduleID: entry.moduleID,
            entityKind: entry.entityKind,
            title: entry.title,
            detail: entry.detail,
            recordedAt: entry.recordedAt,
            entityID: entry.entityID ?? entry.id.uuidString,
            projectIdentity: identity,
            entityRef: entry.entityRef,
            project: nil,
            sourceAppName: entry.sourceAppName,
            sourceKind: entry.sourceKind,
            actionKind: entry.actionKind,
            resumeRef: entry.resumeRef,
            preview: entry.preview,
            resumePayloadJSON: entry.resumePayloadJSON
        )
        let entityRef = migrated.entityRef ?? WorkbenchEntityResolver.resolve(migrated)
        migrated = WorkbenchActivityEntry(
            id: migrated.id,
            kind: migrated.kind,
            moduleID: migrated.moduleID,
            entityKind: migrated.entityKind,
            title: migrated.title,
            detail: migrated.detail,
            recordedAt: migrated.recordedAt,
            entityID: migrated.entityID,
            projectIdentity: migrated.projectIdentity,
            entityRef: entityRef,
            project: nil,
            sourceAppName: migrated.sourceAppName,
            sourceKind: migrated.sourceKind,
            actionKind: migrated.actionKind,
            resumeRef: migrated.resumeRef,
            preview: migrated.preview,
            resumePayloadJSON: migrated.resumePayloadJSON
        )
        return migrated
    }

    static func normalizeForPersist(_ entry: WorkbenchActivityEntry) -> WorkbenchActivityEntry {
        var normalized = migrateEntry(entry)
        if normalized.projectIdentity != nil {
            normalized = WorkbenchActivityEntry(
                id: normalized.id,
                kind: normalized.kind,
                moduleID: normalized.moduleID,
                entityKind: normalized.entityKind,
                title: normalized.title,
                detail: normalized.detail,
                recordedAt: normalized.recordedAt,
                entityID: normalized.entityID,
                projectIdentity: normalized.projectIdentity,
                entityRef: normalized.entityRef ?? WorkbenchEntityResolver.resolve(normalized),
                project: nil,
                sourceAppName: normalized.sourceAppName,
                sourceKind: normalized.sourceKind,
                actionKind: normalized.actionKind,
                resumeRef: normalized.resumeRef,
                preview: normalized.preview,
                resumePayloadJSON: normalized.resumePayloadJSON
            )
        }
        return normalized
    }

    private func persistEntries() {
        Self.persistEntries(entries, to: fileURL)
    }

    private static func persistEntries(_ entries: [WorkbenchActivityEntry], to fileURL: URL) {
        let dir = fileURL.deletingLastPathComponent()
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let envelope = WorkbenchActivityEnvelope(version: schemaVersion, entries: entries)
        guard let data = try? JSONEncoder().encode(envelope) else { return }
        try? data.write(to: fileURL, options: .atomic)
    }
}
