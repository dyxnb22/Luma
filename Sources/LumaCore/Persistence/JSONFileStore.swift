import Foundation

private struct JSONFileEnvelope<Item: Codable & Sendable>: Codable {
    var version: Int
    var items: [Item]
}

public enum JSONFileStoreError: Error, Sendable {
    case notFound
    case writeFailed
}

/// Versioned JSON file persistence with atomic writes and corrupt-file quarantine.
public actor JSONFileStore<Item: Codable & Identifiable & Sendable> {
    private(set) public var items: [Item]
    private let persistenceURL: URL
    private let fileManager: FileManager
    private let schemaVersion: Int

    private var pendingFlushCount = 0
    private var flushTask: Task<Void, Never>?

    public init(
        url: URL,
        schemaVersion: Int = 1
    ) {
        self.persistenceURL = url
        self.fileManager = .default
        self.schemaVersion = schemaVersion
        self.items = []

        try? fileManager.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        if fileManager.fileExists(atPath: url.path) {
            do {
                let data = try Data(contentsOf: url)
                let decoded = try JSONDecoder().decode(JSONFileEnvelope<Item>.self, from: data)
                items = decoded.items
            } catch {
                Self.quarantineCorruptFile(at: url, fileManager: fileManager)
                items = []
            }
        }
    }

    public func persistencePath() -> URL { persistenceURL }

    public func mutate(_ block: (inout [Item]) throws -> Void) throws {
        try block(&items)
        try persistNow()
        pendingFlushCount = 0
        flushTask?.cancel()
        flushTask = nil
    }

    /// Defers disk writes until `flushEvery` mutations or `maxInterval` seconds elapse.
    public func mutateBuffered(
        _ block: (inout [Item]) throws -> Void,
        flushEvery: Int = 10,
        maxInterval: TimeInterval = 30
    ) throws {
        try block(&items)
        pendingFlushCount += 1
        if pendingFlushCount >= flushEvery {
            try persistNow()
            pendingFlushCount = 0
            flushTask?.cancel()
            flushTask = nil
        } else {
            scheduleBufferedFlush(after: maxInterval)
        }
    }

    public func flushIfNeeded() throws {
        guard pendingFlushCount > 0 else { return }
        try persistNow()
        pendingFlushCount = 0
        flushTask?.cancel()
        flushTask = nil
    }

    private func scheduleBufferedFlush(after interval: TimeInterval) {
        flushTask?.cancel()
        flushTask = Task { [weak self] in
            try? await Task.sleep(for: .seconds(interval))
            guard !Task.isCancelled, let self else { return }
            try? await self.flushIfNeeded()
        }
    }

    private func persistNow() throws {
        let payload = JSONFileEnvelope(version: schemaVersion, items: items)
        let data = try JSONEncoder().encode(payload)
        let tempURL = persistenceURL.appendingPathExtension("tmp")
        do {
            try data.write(to: tempURL, options: .atomic)
            if fileManager.fileExists(atPath: persistenceURL.path) {
                try fileManager.removeItem(at: persistenceURL)
            }
            try fileManager.moveItem(at: tempURL, to: persistenceURL)
        } catch {
            try? fileManager.removeItem(at: tempURL)
            throw JSONFileStoreError.writeFailed
        }
    }

    private static func quarantineCorruptFile(at url: URL, fileManager: FileManager) {
        let quarantine = url.deletingPathExtension().appendingPathExtension("corrupt-\(Int(Date().timeIntervalSince1970)).json")
        try? fileManager.moveItem(at: url, to: quarantine)
    }
}
