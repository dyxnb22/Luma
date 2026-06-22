import Foundation
import LumaCore

public enum AppIndexCache {
    public static func load(from url: URL) -> [AppRecord]? {
        guard let data = try? Data(contentsOf: url),
              let decoded = try? JSONDecoder().decode([AppRecord].self, from: data),
              !decoded.isEmpty else { return nil }
        return decoded
    }

    public static func save(_ apps: [AppRecord], to url: URL) {
        try? FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        if let data = try? JSONEncoder().encode(apps) {
            try? data.write(to: url, options: .atomic)
        }
    }

    public static func defaultURL(fileManager: FileManager = .default) -> URL {
        let base = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? fileManager.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support")
        return base.appendingPathComponent("Luma/apps-cache.json")
    }
}
