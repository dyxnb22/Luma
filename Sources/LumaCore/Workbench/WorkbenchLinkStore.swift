import Foundation

/// A link between a project identity and a workbench entity.
public struct WorkbenchProjectLink: Sendable, Hashable, Codable, Identifiable {
    public let id: UUID
    public let stableProjectID: String
    public let entityRef: WorkbenchEntityRef
    public let activityEntryID: UUID?
    public let linkedAt: Date
    public let sourceKind: WorkbenchActivitySourceKind?

    public init(
        id: UUID = UUID(),
        stableProjectID: String,
        entityRef: WorkbenchEntityRef,
        activityEntryID: UUID? = nil,
        linkedAt: Date = Date(),
        sourceKind: WorkbenchActivitySourceKind? = nil
    ) {
        self.id = id
        self.stableProjectID = stableProjectID
        self.entityRef = entityRef
        self.activityEntryID = activityEntryID
        self.linkedAt = linkedAt
        self.sourceKind = sourceKind
    }
}

private struct WorkbenchLinkEnvelope: Codable, Sendable {
    var version: Int
    var links: [WorkbenchProjectLink]
}

/// Local-first project-to-entity link index.
public actor WorkbenchLinkStore {
    public static let shared = WorkbenchLinkStore()
    public static let schemaVersion = 1

    private let fileURL: URL
    private let maxLinks: Int
    private var links: [WorkbenchProjectLink] = []

    public init(
        fileURL: URL = WorkbenchLinkStore.defaultURL(),
        maxLinks: Int = 100
    ) {
        self.fileURL = fileURL
        self.maxLinks = maxLinks
        links = Self.loadLinks(from: fileURL)
    }

    public static func defaultURL(fileManager: FileManager = .default) -> URL {
        let base = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? fileManager.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support")
        return base.appendingPathComponent("Luma/workbench-links.json")
    }

    public func allLinks() -> [WorkbenchProjectLink] {
        links
    }

    public func links(for stableProjectID: String, limit: Int = 10) -> [WorkbenchProjectLink] {
        links
            .filter { $0.stableProjectID == stableProjectID }
            .prefix(max(0, limit))
            .map { $0 }
    }

    public func snapshot(for identity: ProjectIdentity?, limit: Int = 10) -> [WorkbenchProjectLink] {
        guard let identity else { return [] }
        return links(for: identity.stableProjectID, limit: limit)
    }

    public func recordLink(
        stableProjectID: String,
        entityRef: WorkbenchEntityRef,
        activityEntryID: UUID?,
        sourceKind: WorkbenchActivitySourceKind?
    ) {
        if let index = links.firstIndex(where: {
            $0.stableProjectID == stableProjectID && $0.entityRef == entityRef
        }) {
            links.remove(at: index)
        }
        links.insert(
            WorkbenchProjectLink(
                stableProjectID: stableProjectID,
                entityRef: entityRef,
                activityEntryID: activityEntryID,
                sourceKind: sourceKind
            ),
            at: 0
        )
        if links.count > maxLinks {
            links = Array(links.prefix(maxLinks))
        }
        persist()
    }

    public func recordLink(
        for entry: WorkbenchActivityEntry,
        identity: ProjectIdentity
    ) {
        guard let entityRef = WorkbenchEntityResolver.resolve(entry) else { return }
        recordLink(
            stableProjectID: identity.stableProjectID,
            entityRef: entityRef,
            activityEntryID: entry.id,
            sourceKind: entry.sourceKind
        )
    }

    private static func loadLinks(from fileURL: URL) -> [WorkbenchProjectLink] {
        guard let data = try? Data(contentsOf: fileURL) else { return [] }
        let decoder = JSONDecoder()
        if let envelope = try? decoder.decode(WorkbenchLinkEnvelope.self, from: data),
           envelope.version == schemaVersion {
            return envelope.links
        }
        return []
    }

    private func persist() {
        let dir = fileURL.deletingLastPathComponent()
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let envelope = WorkbenchLinkEnvelope(version: Self.schemaVersion, links: links)
        guard let data = try? JSONEncoder().encode(envelope) else { return }
        try? data.write(to: fileURL, options: .atomic)
    }
}

public struct WorkbenchLinkSnapshot: Sendable, Equatable {
    public let currentProjectLinks: [WorkbenchProjectLink]

    public init(currentProjectLinks: [WorkbenchProjectLink] = []) {
        self.currentProjectLinks = currentProjectLinks
    }

    public func enabledLinks(
        enabledModuleIDs: Set<ModuleIdentifier>,
        limit: Int = 3
    ) -> [WorkbenchProjectLink] {
        currentProjectLinks
            .filter { enabledModuleIDs.contains($0.entityRef.moduleID) }
            .prefix(max(0, limit))
            .map { $0 }
    }
}
