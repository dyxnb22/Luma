import Foundation

public struct ClipboardEntry: Identifiable, Sendable, Hashable, Codable {
    public let id: UUID
    public var text: String
    public var createdAt: Date
    public var isPinned: Bool
    public var detectedKind: ClipboardEntryKind
    public var sourceAppName: String?
    public var sourceBundleID: String?
    public var imageData: Data?
    public var imagePasteboardType: String?

    public init(
        id: UUID = UUID(),
        text: String,
        createdAt: Date = Date(),
        isPinned: Bool = false,
        detectedKind: ClipboardEntryKind? = nil,
        sourceAppName: String? = nil,
        sourceBundleID: String? = nil,
        imageData: Data? = nil,
        imagePasteboardType: String? = nil
    ) {
        self.id = id
        self.text = text
        self.createdAt = createdAt
        self.isPinned = isPinned
        self.detectedKind = detectedKind ?? ClipboardEntryKind.detect(from: text, pasteboardTypes: [])
        self.sourceAppName = sourceAppName
        self.sourceBundleID = sourceBundleID
        self.imageData = imageData
        self.imagePasteboardType = imagePasteboardType
    }

    public var displayText: String {
        if detectedKind == .image, let imageData, !imageData.isEmpty {
            let kb = max(1, imageData.count / 1024)
            return "[Image \(kb)KB]"
        }
        return text
    }

    enum CodingKeys: String, CodingKey {
        case id, text, createdAt, isPinned, detectedKind, sourceAppName, sourceBundleID
        case imageData, imagePasteboardType
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(UUID.self, forKey: .id)
        text = try container.decode(String.self, forKey: .text)
        createdAt = try container.decode(Date.self, forKey: .createdAt)
        isPinned = try container.decode(Bool.self, forKey: .isPinned)
        imageData = try container.decodeIfPresent(Data.self, forKey: .imageData)
        imagePasteboardType = try container.decodeIfPresent(String.self, forKey: .imagePasteboardType)
        let types = imagePasteboardType.map { [$0] } ?? []
        detectedKind = try container.decodeIfPresent(ClipboardEntryKind.self, forKey: .detectedKind)
            ?? ClipboardEntryKind.detect(from: text, pasteboardTypes: types)
        sourceAppName = try container.decodeIfPresent(String.self, forKey: .sourceAppName)
        sourceBundleID = try container.decodeIfPresent(String.self, forKey: .sourceBundleID)
    }
}

public struct ClipboardStatistics: Sendable, Equatable {
    public let total: Int
    public let pinned: Int

    public init(total: Int, pinned: Int) {
        self.total = total
        self.pinned = pinned
    }
}

public enum ClipboardStoreChangeHub {
    private static let lock = NSLock()
    private nonisolated(unsafe) static var continuations: [UUID: AsyncStream<Void>.Continuation] = [:]

    public static func dataChanges() -> AsyncStream<Void> {
        AsyncStream { continuation in
            let id = UUID()
            lock.lock()
            continuations[id] = continuation
            lock.unlock()
            continuation.onTermination = { _ in
                lock.lock()
                continuations.removeValue(forKey: id)
                lock.unlock()
            }
        }
    }

    public static func publishDataChanged() {
        lock.lock()
        let targets = continuations.values
        lock.unlock()
        for continuation in targets {
            continuation.yield()
        }
    }
}

public actor ClipboardHistoryStore {
    private var entries: [ClipboardEntry] = []
    private var maxEntries: Int
    private var maxAge: TimeInterval
    private var maxTextBytes: Int
    private let persistenceURL: URL?

    public init(maxEntries: Int = 500, maxAge: TimeInterval = 7 * 24 * 60 * 60, maxTextBytes: Int = 100 * 1024, persistenceURL: URL? = nil) {
        self.maxEntries = maxEntries
        self.maxAge = maxAge
        self.maxTextBytes = maxTextBytes
        self.persistenceURL = persistenceURL
        if let persistenceURL {
            do {
                let data = try Data(contentsOf: persistenceURL)
                self.entries = try JSONDecoder().decode([ClipboardEntry].self, from: data)
                self.entries = Self.pruned(entries: entries, maxEntries: maxEntries, maxAge: maxAge, now: Date())
                Self.persist(entries: entries, to: persistenceURL)
            } catch {
                self.entries = []
                Self.quarantineCorruptFile(at: persistenceURL)
            }
        }
    }

    public func add(
        text: String,
        types: [String],
        now: Date = Date(),
        sourceAppName: String? = nil,
        sourceBundleID: String? = nil,
        imageData: Data? = nil,
        imagePasteboardType: String? = nil
    ) {
        let isImage = ClipboardEntryKind.isImageTypes(types)
        if !isImage {
            guard !ClipboardFilter.shouldSkip(types: types), ClipboardFilter.acceptsText(text, maxBytes: maxTextBytes) else { return }
        } else {
            guard !ClipboardFilter.shouldSkip(types: types), imageData != nil else { return }
        }
        if isImage {
            entries.removeAll { $0.imageData == imageData && $0.detectedKind == .image }
        } else {
            entries.removeAll { $0.text == text }
        }
        entries.insert(
            ClipboardEntry(
                text: text,
                createdAt: now,
                detectedKind: ClipboardEntryKind.detect(from: text, pasteboardTypes: types),
                sourceAppName: sourceAppName,
                sourceBundleID: sourceBundleID,
                imageData: imageData,
                imagePasteboardType: imagePasteboardType
            ),
            at: 0
        )
        prune(now: now)
        persist()
    }

    public func search(_ query: String, limit: Int = 20) -> [ClipboardEntry] {
        list(filter: .all, query: query, limit: limit)
    }

    public func list(filter: ClipboardListFilter, query: String = "", limit: Int = 50) -> [ClipboardEntry] {
        pruneAndPersistIfNeeded(now: Date())
        let sorted = entries.sorted {
            if $0.isPinned != $1.isPinned { return $0.isPinned && !$1.isPinned }
            return $0.createdAt > $1.createdAt
        }
        let filtered = sorted.filter { entry in
            guard matchesFilter(entry, filter: filter) else { return false }
            return matchesQuery(entry, query: query)
        }
        return Array(filtered.prefix(limit))
    }

    public func statistics() -> ClipboardStatistics {
        pruneAndPersistIfNeeded(now: Date())
        let pinned = entries.filter(\.isPinned).count
        return ClipboardStatistics(total: entries.count, pinned: pinned)
    }

    public func updateRetention(maxEntries: Int, maxAge: TimeInterval, maxTextBytes: Int, now: Date = Date()) {
        self.maxEntries = max(1, maxEntries)
        self.maxAge = max(1, maxAge)
        self.maxTextBytes = max(1, maxTextBytes)
        prune(now: now)
        persist()
    }

    public func pin(_ id: UUID, isPinned: Bool = true) {
        guard let index = entries.firstIndex(where: { $0.id == id }) else { return }
        entries[index].isPinned = isPinned
        persist()
    }

    public func clear() {
        entries.removeAll()
        persist()
    }

    public func clearUnpinned() {
        entries.removeAll { !$0.isPinned }
        persist()
    }

    public func removeEntry(_ id: UUID) {
        entries.removeAll { $0.id == id }
        persist()
    }

    private func matchesFilter(_ entry: ClipboardEntry, filter: ClipboardListFilter) -> Bool {
        switch filter {
        case .all:
            return true
        case .pinned:
            return entry.isPinned
        case .image:
            return entry.detectedKind == .image
        }
    }

    private func matchesQuery(_ entry: ClipboardEntry, query: String) -> Bool {
        let normalized = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !normalized.isEmpty else { return true }
        let tokens = normalized.split(whereSeparator: \.isWhitespace).map(String.init)
        guard !tokens.isEmpty else { return true }
        let haystack = entry.text.lowercased()
        return tokens.allSatisfy { haystack.contains($0) }
    }

    private func prune(now: Date) {
        entries = Self.pruned(entries: entries, maxEntries: maxEntries, maxAge: maxAge, now: now)
    }

    private func pruneAndPersistIfNeeded(now: Date) {
        let previous = entries
        prune(now: now)
        if previous != entries {
            persist()
        }
    }

    private func persist() {
        guard let persistenceURL else { return }
        Self.persist(entries: entries, to: persistenceURL)
        ClipboardStoreChangeHub.publishDataChanged()
    }

    private static func pruned(entries: [ClipboardEntry], maxEntries: Int, maxAge: TimeInterval, now: Date) -> [ClipboardEntry] {
        let cutoff = now.addingTimeInterval(-maxAge)
        var pruned = entries.filter { $0.isPinned || $0.createdAt >= cutoff }
        if pruned.count > maxEntries {
            let pinned = pruned.filter(\.isPinned)
            let unpinned = pruned.filter { !$0.isPinned }.prefix(max(0, maxEntries - pinned.count))
            pruned = pinned + unpinned
        }
        return pruned
    }

    private static func persist(entries: [ClipboardEntry], to persistenceURL: URL) {
        try? FileManager.default.createDirectory(at: persistenceURL.deletingLastPathComponent(), withIntermediateDirectories: true)
        if let data = try? JSONEncoder().encode(entries) {
            try? data.write(to: persistenceURL, options: .atomic)
        }
    }

    private static func quarantineCorruptFile(at url: URL) {
        let fm = FileManager.default
        guard fm.fileExists(atPath: url.path) else { return }
        let ts = Int(Date().timeIntervalSince1970)
        let backup = url.deletingLastPathComponent()
            .appendingPathComponent(url.lastPathComponent + ".corrupt-\(ts).bak")
        try? fm.moveItem(at: url, to: backup)
    }
}
