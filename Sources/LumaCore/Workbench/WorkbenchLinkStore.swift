import Foundation

/// A link between a project identity and a workbench entity.
public struct WorkbenchProjectLink: Sendable, Hashable, Codable, Identifiable {
    public let id: UUID
    public let stableProjectID: String
    public let entityRef: WorkbenchEntityRef
    public let activityEntryID: UUID?
    public let linkedAt: Date
    public let sourceKind: WorkbenchActivitySourceKind?
    /// Label-only project key for legacy unmatched migration matching.
    public let labelFallback: String?

    public init(
        id: UUID = UUID(),
        stableProjectID: String,
        entityRef: WorkbenchEntityRef,
        activityEntryID: UUID? = nil,
        linkedAt: Date = Date(),
        sourceKind: WorkbenchActivitySourceKind? = nil,
        labelFallback: String? = nil
    ) {
        self.id = id
        self.stableProjectID = stableProjectID
        self.entityRef = entityRef
        self.activityEntryID = activityEntryID
        self.linkedAt = linkedAt
        self.sourceKind = sourceKind
        self.labelFallback = labelFallback
    }
}

private struct WorkbenchLinkEnvelope: Codable, Sendable {
    var version: Int
    var links: [WorkbenchProjectLink]
}

/// Shared rules for deriving project links from activity entries.
public enum WorkbenchLinkIndexing {
    public static func isLinkEligible(_ entry: WorkbenchActivityEntry) -> Bool {
        guard entry.projectIdentity != nil else { return false }
        return WorkbenchEntityResolver.resolve(entry) != nil
    }

    public static func dedupeKey(stableProjectID: String, entityRef: WorkbenchEntityRef) -> String {
        "\(stableProjectID)|\(entityRef.kind.rawValue)|\(entityRef.entityID)"
    }

    public static func dedupeKey(for link: WorkbenchProjectLink) -> String {
        dedupeKey(stableProjectID: link.stableProjectID, entityRef: link.entityRef)
    }
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
        return links(matching: identity, limit: limit)
    }

    public func backfillFromActivitiesIfEmpty(_ entries: [WorkbenchActivityEntry]) {
        ensureLinksIndexed(for: nil, from: entries)
    }

    public func ensureLinksIndexed(
        for identity: ProjectIdentity?,
        from entries: [WorkbenchActivityEntry]
    ) {
        if links.isEmpty {
            applyBackfill(collectLinks(from: entries, limit: maxLinks))
            return
        }
        guard let identity else { return }
        guard snapshot(for: identity, limit: 1).isEmpty else { return }
        let projectEntries = WorkbenchActivityQuery.recent(
            for: identity,
            entries: entries,
            limit: entries.count
        )
        let collected = collectLinks(from: projectEntries, limit: maxLinks)
        guard !collected.isEmpty else { return }
        mergeBackfill(collected)
    }

    private func collectLinks(
        from entries: [WorkbenchActivityEntry],
        limit: Int
    ) -> [WorkbenchProjectLink] {
        var seen = Set(links.map(WorkbenchLinkIndexing.dedupeKey(for:)))
        var collected: [WorkbenchProjectLink] = []
        for entry in entries {
            guard WorkbenchLinkIndexing.isLinkEligible(entry) else { continue }
            guard let identity = entry.projectIdentity else { continue }
            guard let entityRef = WorkbenchEntityResolver.resolve(entry) else { continue }
            let key = WorkbenchLinkIndexing.dedupeKey(
                stableProjectID: identity.stableProjectID,
                entityRef: entityRef
            )
            guard seen.insert(key).inserted else { continue }
            collected.append(WorkbenchProjectLink(
                stableProjectID: identity.stableProjectID,
                entityRef: entityRef,
                activityEntryID: entry.id,
                linkedAt: entry.recordedAt,
                sourceKind: entry.sourceKind,
                labelFallback: identity.matchedPath == nil ? identity.labelFallback : nil
            ))
            if collected.count >= limit { break }
        }
        return collected
    }

    private func applyBackfill(_ collected: [WorkbenchProjectLink]) {
        guard !collected.isEmpty else { return }
        links = collected.sorted { $0.linkedAt > $1.linkedAt }
        persist()
    }

    private func mergeBackfill(_ collected: [WorkbenchProjectLink]) {
        links.append(contentsOf: collected)
        links.sort { $0.linkedAt > $1.linkedAt }
        var seen = Set<String>()
        links = links.filter { seen.insert(WorkbenchLinkIndexing.dedupeKey(for: $0)).inserted }
        if links.count > maxLinks {
            links = Array(links.prefix(maxLinks))
        }
        persist()
    }

    public func recordLink(
        stableProjectID: String,
        entityRef: WorkbenchEntityRef,
        activityEntryID: UUID?,
        sourceKind: WorkbenchActivitySourceKind?,
        labelFallback: String? = nil,
        linkedAt: Date = Date()
    ) {
        let key = WorkbenchLinkIndexing.dedupeKey(stableProjectID: stableProjectID, entityRef: entityRef)
        if let index = links.firstIndex(where: {
            WorkbenchLinkIndexing.dedupeKey(for: $0) == key
        }) {
            links.remove(at: index)
        }
        links.insert(
            WorkbenchProjectLink(
                stableProjectID: stableProjectID,
                entityRef: entityRef,
                activityEntryID: activityEntryID,
                linkedAt: linkedAt,
                sourceKind: sourceKind,
                labelFallback: labelFallback
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
        guard WorkbenchLinkIndexing.isLinkEligible(entry),
              let entityRef = WorkbenchEntityResolver.resolve(entry) else { return }
        recordLink(
            stableProjectID: identity.stableProjectID,
            entityRef: entityRef,
            activityEntryID: entry.id,
            sourceKind: entry.sourceKind,
            labelFallback: identity.matchedPath == nil ? identity.labelFallback : nil,
            linkedAt: entry.recordedAt
        )
    }

    private func links(matching identity: ProjectIdentity, limit: Int) -> [WorkbenchProjectLink] {
        links
            .filter { Self.matchesProject($0, identity: identity) }
            .prefix(max(0, limit))
            .map { $0 }
    }

    private static func matchesProject(_ link: WorkbenchProjectLink, identity: ProjectIdentity) -> Bool {
        if link.stableProjectID == identity.stableProjectID {
            return true
        }
        if identity.matchedPath == nil,
           let label = link.labelFallback,
           label == identity.labelFallback,
           ProjectIdentity.isLegacyLabelStableID(link.stableProjectID) {
            return true
        }
        return false
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
