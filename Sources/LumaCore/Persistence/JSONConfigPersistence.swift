import Foundation

/// Atomic JSON read/write with corrupt-file quarantine for singleton config files.
public enum JSONConfigPersistence {
    public struct LoadResult<T: Sendable>: Sendable {
        public let value: T
        public let wasCorrupt: Bool
        public let quarantinePath: String?

        public init(value: T, wasCorrupt: Bool = false, quarantinePath: String? = nil) {
            self.value = value
            self.wasCorrupt = wasCorrupt
            self.quarantinePath = quarantinePath
        }
    }

    public static func load<T: Decodable>(
        from url: URL,
        fallback: T,
        fileManager: FileManager = .default,
        decoder: JSONDecoder = JSONDecoder()
    ) -> LoadResult<T> {
        guard fileManager.fileExists(atPath: url.path) else {
            return LoadResult(value: fallback)
        }
        guard let data = try? Data(contentsOf: url) else {
            return LoadResult(value: fallback)
        }
        do {
            return LoadResult(value: try decoder.decode(T.self, from: data))
        } catch {
            let quarantine = quarantineCorruptFile(at: url, fileManager: fileManager)
            ConfigCorruptionRegistry.record(fileName: url.lastPathComponent)
            if let quarantine {
                CrashLogRecording.record("config.corrupt file=\(url.lastPathComponent) quarantine=\(quarantine.lastPathComponent)")
            }
            return LoadResult(value: fallback, wasCorrupt: true, quarantinePath: quarantine?.path)
        }
    }

    public static func save<T: Encodable>(
        _ value: T,
        to url: URL,
        fileManager: FileManager = .default,
        encoder: JSONEncoder = JSONEncoder()
    ) throws {
        let directory = url.deletingLastPathComponent()
        try fileManager.createDirectory(at: directory, withIntermediateDirectories: true)
        let data = try encoder.encode(value)
        let tempURL = url.appendingPathExtension("tmp")
        try data.write(to: tempURL, options: .atomic)
        do {
            if fileManager.fileExists(atPath: url.path) {
                try fileManager.replaceItem(at: url, withItemAt: tempURL, backupItemName: nil, options: [], resultingItemURL: nil)
            } else {
                try fileManager.moveItem(at: tempURL, to: url)
            }
        } catch {
            try? fileManager.removeItem(at: tempURL)
            throw error
        }
    }

    @discardableResult
    public static func quarantineCorruptFile(at url: URL, fileManager: FileManager = .default) -> URL? {
        let quarantine = url.deletingPathExtension()
            .appendingPathExtension("corrupt-\(Int(Date().timeIntervalSince1970)).json")
        do {
            try fileManager.moveItem(at: url, to: quarantine)
            return quarantine
        } catch {
            return nil
        }
    }
}
