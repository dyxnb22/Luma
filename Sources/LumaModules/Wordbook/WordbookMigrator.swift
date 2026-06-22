import Foundation

/// One-way migration of the original wordbot SQLite into Luma's Application Support directory.
///
/// After migration the Luma-owned copy is the single source of truth. The wordbot directory is
/// untouched and can be deleted by the user once they confirm the new location works (see ADR-009).
public enum WordbookMigrator {
    public static let defaultSourcePath = "/Users/diaoyuxuan/wordbot/data/wordpet.sqlite3"

    public static func defaultDestinationURL(fileManager: FileManager = .default) -> URL {
        let base = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? fileManager.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support")
        return base
            .appendingPathComponent("Luma", isDirectory: true)
            .appendingPathComponent("Wordbook", isDirectory: true)
            .appendingPathComponent("wordpet.sqlite3", isDirectory: false)
    }

    public enum Result: Sendable, Equatable {
        case alreadyMigrated
        case copied(from: URL, to: URL)
        case sourceMissing
    }

    /// Idempotent: returns `.alreadyMigrated` if destination already exists. Otherwise copies the
    /// source DB. Does **not** overwrite an existing destination, ever.
    @discardableResult
    public static func migrateIfNeeded(
        source: URL = URL(fileURLWithPath: defaultSourcePath),
        destination: URL = defaultDestinationURL(),
        fileManager: FileManager = .default
    ) throws -> Result {
        if fileManager.fileExists(atPath: destination.path) {
            return .alreadyMigrated
        }
        guard fileManager.fileExists(atPath: source.path) else {
            return .sourceMissing
        }
        try fileManager.createDirectory(
            at: destination.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        try fileManager.copyItem(at: source, to: destination)
        return .copied(from: source, to: destination)
    }
}
