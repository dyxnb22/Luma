import Foundation

public struct ClipboardEntry: Identifiable, Sendable, Hashable, Codable {
    public let id: UUID
    public var text: String
    public var createdAt: Date
    public var isPinned: Bool

    public init(id: UUID = UUID(), text: String, createdAt: Date = Date(), isPinned: Bool = false) {
        self.id = id
        self.text = text
        self.createdAt = createdAt
        self.isPinned = isPinned
    }
}

public actor ClipboardHistoryStore {
    private var entries: [ClipboardEntry] = []
    private let maxEntries: Int
    private let maxAge: TimeInterval
    private let maxTextBytes: Int
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
            } catch {
                self.entries = []
                Self.quarantineCorruptFile(at: persistenceURL)
            }
        }
    }

    public func add(text: String, types: [String], now: Date = Date()) {
        guard !ClipboardFilter.shouldSkip(types: types), ClipboardFilter.acceptsText(text, maxBytes: maxTextBytes) else { return }
        entries.removeAll { $0.text == text }
        entries.insert(ClipboardEntry(text: text, createdAt: now), at: 0)
        prune(now: now)
        persist()
    }

    public func search(_ query: String, limit: Int = 20) -> [ClipboardEntry] {
        let normalized = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        let source = entries.sorted {
            if $0.isPinned != $1.isPinned { return $0.isPinned && !$1.isPinned }
            return $0.createdAt > $1.createdAt
        }
        guard !normalized.isEmpty else { return Array(source.prefix(limit)) }
        return Array(source.filter { $0.text.lowercased().contains(normalized) }.prefix(limit))
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

    public func removeEntry(_ id: UUID) {
        entries.removeAll { $0.id == id }
        persist()
    }

    private func prune(now: Date) {
        let cutoff = now.addingTimeInterval(-maxAge)
        entries.removeAll { !$0.isPinned && $0.createdAt < cutoff }
        if entries.count > maxEntries {
            let pinned = entries.filter(\.isPinned)
            let unpinned = entries.filter { !$0.isPinned }.prefix(max(0, maxEntries - pinned.count))
            entries = pinned + unpinned
        }
    }

    private func persist() {
        guard let persistenceURL else { return }
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
