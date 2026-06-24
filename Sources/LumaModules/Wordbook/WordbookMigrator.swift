import Foundation

/// One-way migration of the original wordbot SQLite into Luma's Application Support directory.
///
/// After migration the Luma-owned copy is the single source of truth. The wordbot directory is
/// untouched and can be deleted by the user once they confirm the new location works (see ADR-009).
public enum WordbookMigrator {
    public static let defaultSourcePath = "/Users/diaoyuxuan/wordbot/data/wordpet.sqlite3"

    public enum MigrationNotice: Sendable, Equatable {
        case none
        case sourceMissing
        case failed
    }

    private static let noticeLock = NSLock()
    private nonisolated(unsafe) static var _migrationNotice: MigrationNotice = .none

    public static var migrationNotice: MigrationNotice {
        noticeLock.lock()
        defer { noticeLock.unlock() }
        return _migrationNotice
    }

    public static func setMigrationNotice(_ notice: MigrationNotice) {
        noticeLock.lock()
        _migrationNotice = notice
        noticeLock.unlock()
    }

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
            setMigrationNotice(.none)
            return .alreadyMigrated
        }
        guard fileManager.fileExists(atPath: source.path) else {
            setMigrationNotice(.sourceMissing)
            return .sourceMissing
        }
        try fileManager.createDirectory(
            at: destination.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        try fileManager.copyItem(at: source, to: destination)
        setMigrationNotice(.none)
        return .copied(from: source, to: destination)
    }
}
