import Foundation

public struct AppActivationRecord: Codable, Sendable, Equatable {
    public let bundleID: String
    public var activationCount: Int
    public var lastActivated: Date

    public init(bundleID: String, activationCount: Int, lastActivated: Date) {
        self.bundleID = bundleID
        self.activationCount = activationCount
        self.lastActivated = lastActivated
    }
}

public actor AppActivationTracker {
    private let url: URL
    private let coalesceWindow: Duration
    private var records: [String: AppActivationRecord]
    private var persistTask: Task<Void, Never>?
    private var hasUnflushedChanges = false

    public init(url: URL, coalesceWindow: Duration = .seconds(1)) {
        self.url = url
        self.coalesceWindow = coalesceWindow
        self.records = (try? Self.load(from: url)) ?? [:]
    }

    public static func defaultTracker(fileManager: FileManager = .default) -> AppActivationTracker {
        let base = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? fileManager.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support")
        return AppActivationTracker(url: base.appendingPathComponent("Luma/app-activations.json"))
    }

    public func record(bundleID: String, at date: Date = Date()) {
        var record = records[bundleID] ?? AppActivationRecord(bundleID: bundleID, activationCount: 0, lastActivated: date)
        record.activationCount += 1
        record.lastActivated = date
        records[bundleID] = record
        hasUnflushedChanges = true
        if coalesceWindow == .zero {
            persistNow()
        } else {
            schedulePersist()
        }
    }

    public func flush() {
        persistTask?.cancel()
        persistTask = nil
        if hasUnflushedChanges {
            persistNow()
        }
    }

    public func rankedBundleIDs(from candidates: [String], at now: Date = Date()) -> [String] {
        candidates.sorted { lhs, rhs in
            score(for: lhs, at: now) > score(for: rhs, at: now)
        }
    }

    private func score(for bundleID: String, at now: Date) -> Double {
        let record = records[bundleID]
        let lastUsed = record?.lastActivated ?? .distantPast
        let count = record?.activationCount ?? 0
        let elapsed = now.timeIntervalSince(lastUsed)
        let recencyScore = exp(-elapsed / 3600)
        let frequencyScore = log(1 + Double(count)) / log(1 + 50)
        return recencyScore * 0.6 + frequencyScore * 0.4
    }

    private func schedulePersist() {
        persistTask?.cancel()
        let window = coalesceWindow
        persistTask = Task { [weak self] in
            try? await Task.sleep(for: window)
            guard !Task.isCancelled else { return }
            await self?.persistNow()
        }
    }

    private func persistNow() {
        try? FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        let encoded = Array(records.values)
        if let data = try? JSONEncoder().encode(encoded) {
            try? data.write(to: url, options: .atomic)
            hasUnflushedChanges = false
        }
    }

    private static func load(from url: URL) throws -> [String: AppActivationRecord] {
        let data = try Data(contentsOf: url)
        let decoded = try JSONDecoder().decode([AppActivationRecord].self, from: data)
        return Dictionary(uniqueKeysWithValues: decoded.map { ($0.bundleID, $0) })
    }
}
