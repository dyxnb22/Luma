import Foundation

public actor CommandUsageTracker: Sendable {
    private let url: URL
    private var counts: [String: Int]

    public init(url: URL) {
        self.url = url
        self.counts = (try? Self.load(from: url)) ?? [:]
    }

    public static func defaultTracker(fileManager: FileManager = .default) -> CommandUsageTracker {
        let base = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? fileManager.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support")
        return CommandUsageTracker(url: base.appendingPathComponent("Luma/command-usage.json"))
    }

    public func snapshot() -> [String: Int] {
        counts
    }

    public func record(trigger: String, at date: Date = Date()) {
        let key = trigger.lowercased()
        guard !key.isEmpty else { return }
        counts[key, default: 0] += 1
        _ = date
        persist()
    }

    private func persist() {
        try? FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        if let data = try? JSONEncoder().encode(counts) {
            try? data.write(to: url, options: .atomic)
        }
    }

    private static func load(from url: URL) throws -> [String: Int] {
        let data = try Data(contentsOf: url)
        return try JSONDecoder().decode([String: Int].self, from: data)
    }
}
