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
    public var fileURLs: [String]?
    public var colorHex: String?
    public var contentHash: String

    public init(
        id: UUID = UUID(),
        text: String,
        createdAt: Date = Date(),
        isPinned: Bool = false,
        detectedKind: ClipboardEntryKind? = nil,
        sourceAppName: String? = nil,
        sourceBundleID: String? = nil,
        imageData: Data? = nil,
        imagePasteboardType: String? = nil,
        fileURLs: [String]? = nil,
        colorHex: String? = nil,
        contentHash: String? = nil
    ) {
        self.id = id
        self.text = text
        self.createdAt = createdAt
        self.isPinned = isPinned
        self.fileURLs = fileURLs
        self.colorHex = colorHex
        let resolvedKind = detectedKind
            ?? ClipboardEntryKind.detect(from: text, pasteboardTypes: [], fileURLs: fileURLs)
        self.detectedKind = resolvedKind
        self.sourceAppName = sourceAppName
        self.sourceBundleID = sourceBundleID
        self.imageData = imageData
        self.imagePasteboardType = imagePasteboardType
        self.contentHash = contentHash
            ?? ClipboardContentHash.compute(
                text: text,
                imageData: imageData,
                fileURLs: fileURLs,
                colorHex: colorHex
            )
    }

    public var displayText: String {
        if detectedKind == .image, let imageData, !imageData.isEmpty {
            let kb = max(1, imageData.count / 1024)
            return "[Image \(kb)KB]"
        }
        if detectedKind == .file, let fileURLs, !fileURLs.isEmpty {
            if fileURLs.count == 1 {
                return URL(fileURLWithPath: fileURLs[0]).lastPathComponent
            }
            return "[\(fileURLs.count) files]"
        }
        if detectedKind == .color, let colorHex {
            return colorHex
        }
        return text
    }

    enum CodingKeys: String, CodingKey {
        case id, text, createdAt, isPinned, detectedKind, sourceAppName, sourceBundleID
        case imageData, imagePasteboardType, fileURLs, colorHex, contentHash
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(UUID.self, forKey: .id)
        text = try container.decode(String.self, forKey: .text)
        createdAt = try container.decode(Date.self, forKey: .createdAt)
        isPinned = try container.decode(Bool.self, forKey: .isPinned)
        imageData = try container.decodeIfPresent(Data.self, forKey: .imageData)
        imagePasteboardType = try container.decodeIfPresent(String.self, forKey: .imagePasteboardType)
        fileURLs = try container.decodeIfPresent([String].self, forKey: .fileURLs)
        colorHex = try container.decodeIfPresent(String.self, forKey: .colorHex)
        let types = imagePasteboardType.map { [$0] } ?? []
        detectedKind = try container.decodeIfPresent(ClipboardEntryKind.self, forKey: .detectedKind)
            ?? ClipboardEntryKind.detect(from: text, pasteboardTypes: types, fileURLs: fileURLs)
        sourceAppName = try container.decodeIfPresent(String.self, forKey: .sourceAppName)
        sourceBundleID = try container.decodeIfPresent(String.self, forKey: .sourceBundleID)
        let decodedHash = try container.decodeIfPresent(String.self, forKey: .contentHash) ?? ""
        if decodedHash.isEmpty {
            contentHash = ClipboardContentHash.compute(
                text: text,
                imageData: imageData,
                fileURLs: fileURLs,
                colorHex: colorHex
            )
        } else {
            contentHash = decodedHash
        }
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
    private var ignoredBundleIDs: Set<String> = []
    private let persistenceURL: URL?

    public init(maxEntries: Int = 500, maxAge: TimeInterval = 7 * 24 * 60 * 60, maxTextBytes: Int = 100 * 1024, persistenceURL: URL? = nil) {
        self.maxEntries = maxEntries
        self.maxAge = maxAge
        self.maxTextBytes = maxTextBytes
        self.persistenceURL = persistenceURL
        if let persistenceURL {
            do {
                self.entries = try ClipboardHistoryPersistence.load(from: persistenceURL)
                self.entries = Self.pruned(entries: entries, maxEntries: maxEntries, maxAge: maxAge, now: Date())
                ClipboardHistoryPersistence.save(entries: entries, to: persistenceURL)
            } catch {
                self.entries = []
                Self.quarantineCorruptFile(at: persistenceURL)
            }
        }
    }

    public func updateCapturePolicy(ignoredBundleIDs: Set<String>) {
        self.ignoredBundleIDs = ignoredBundleIDs
    }

    public func add(
        text: String,
        types: [String],
        now: Date = Date(),
        sourceAppName: String? = nil,
        sourceBundleID: String? = nil,
        imageData: Data? = nil,
        imagePasteboardType: String? = nil,
        fileURLs: [String]? = nil
    ) {
        if ClipboardFilter.shouldSkipSource(bundleID: sourceBundleID, ignoredBundleIDs: ignoredBundleIDs) {
            return
        }
        if ClipboardFilter.shouldSkip(types: types) {
            return
        }

        let isImage = ClipboardEntryKind.isImageTypes(types) && imageData != nil
        let isFile = ClipboardEntryKind.isFileTypes(types) && fileURLs?.isEmpty == false

        if isFile {
            guard let fileURLs, !fileURLs.isEmpty else { return }
        } else if isImage {
            guard imageData != nil else { return }
        } else {
            guard ClipboardFilter.acceptsText(text, maxBytes: maxTextBytes) else { return }
            guard !ClipboardFilter.looksSensitiveText(text) else { return }
        }

        let colorHex = (!isImage && !isFile) ? ClipboardEntryKind.normalizedColorHex(from: text) : nil
        let kind = ClipboardEntryKind.detect(from: text, pasteboardTypes: types, fileURLs: fileURLs)
        let hash = ClipboardContentHash.compute(
            text: text,
            imageData: imageData,
            fileURLs: fileURLs,
            colorHex: colorHex
        )
        entries.removeAll { $0.contentHash == hash }

        entries.insert(
            ClipboardEntry(
                text: text,
                createdAt: now,
                detectedKind: kind,
                sourceAppName: sourceAppName,
                sourceBundleID: sourceBundleID,
                imageData: imageData,
                imagePasteboardType: imagePasteboardType,
                fileURLs: fileURLs,
                colorHex: colorHex,
                contentHash: hash
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

    public func clearRecent(window: ClipboardRecentClearWindow, now: Date = Date(), calendar: Calendar = .current) {
        let cutoff = window.cutoff(from: now, calendar: calendar)
        switch window {
        case .today:
            entries.removeAll { !$0.isPinned && $0.createdAt >= cutoff && $0.createdAt <= now }
        case .last5Minutes, .lastHour:
            entries.removeAll { !$0.isPinned && $0.createdAt >= cutoff }
        }
        persist()
    }

    public func removeEntry(_ id: UUID) {
        entries.removeAll { $0.id == id }
        persist()
    }

    public func entry(id: UUID) -> ClipboardEntry? {
        entries.first { $0.id == id }
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

    private struct QueryParts {
        let kindFilter: ClipboardEntryKind?
        let tokens: [String]
    }

    private func parseQuery(_ query: String) -> QueryParts {
        let normalized = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !normalized.isEmpty else { return QueryParts(kindFilter: nil, tokens: []) }

        let kindPrefixes: [(String, ClipboardEntryKind)] = [
            ("img:", .image),
            ("image:", .image),
            ("link:", .link),
            ("code:", .code),
            ("file:", .file),
            ("color:", .color),
            ("email:", .email)
        ]
        for (prefix, kind) in kindPrefixes {
            if normalized.hasPrefix(prefix) {
                let remainder = String(normalized.dropFirst(prefix.count)).trimmingCharacters(in: .whitespacesAndNewlines)
                let tokens = remainder.split(whereSeparator: \.isWhitespace).map(String.init)
                return QueryParts(kindFilter: kind, tokens: tokens)
            }
        }

        let tokens = normalized.split(whereSeparator: \.isWhitespace).map(String.init)
        return QueryParts(kindFilter: nil, tokens: tokens)
    }

    private func matchesQuery(_ entry: ClipboardEntry, query: String) -> Bool {
        let parts = parseQuery(query)
        if let kindFilter = parts.kindFilter, entry.detectedKind != kindFilter {
            return false
        }
        guard !parts.tokens.isEmpty else { return true }
        let haystack = entry.searchHaystack()
        return parts.tokens.allSatisfy { haystack.contains($0) }
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
        ClipboardHistoryPersistence.save(entries: entries, to: persistenceURL)
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

    private static func quarantineCorruptFile(at url: URL) {
        let fm = FileManager.default
        guard fm.fileExists(atPath: url.path) else { return }
        let ts = Int(Date().timeIntervalSince1970)
        let backup = url.deletingLastPathComponent()
            .appendingPathComponent(url.lastPathComponent + ".corrupt-\(ts).bak")
        try? fm.moveItem(at: url, to: backup)
    }
}
