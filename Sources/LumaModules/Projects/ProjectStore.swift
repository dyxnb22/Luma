import Foundation
import LumaCore

public actor ProjectStore {
    private let fileURL: URL
    private let fileManager: FileManager
    private var config: ProjectsConfig

    private var lastLoadWasCorrupt = false

    public init(fileURL: URL = ProjectStore.defaultFileURL, fileManager: FileManager = .default) {
        self.fileURL = fileURL
        self.fileManager = fileManager
        let result = JSONConfigPersistence.load(from: fileURL, fallback: ProjectsConfig.empty, fileManager: fileManager)
        self.config = result.value
        self.lastLoadWasCorrupt = result.wasCorrupt
    }

    public static var defaultFileURL: URL {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support")
        return base.appendingPathComponent("Luma/projects.json", isDirectory: false)
    }

    public func current() -> ProjectsConfig {
        config
    }

    public func loadWasCorrupt() -> Bool {
        lastLoadWasCorrupt
    }

    public func save(_ updated: ProjectsConfig) throws {
        try JSONConfigPersistence.save(updated, to: fileURL, fileManager: fileManager)
        config = updated
    }

    public func recordOpened(path: String) throws {
        var updated = config
        var recent = updated.recent.filter { $0 != path }
        recent.insert(path, at: 0)
        updated.recent = Array(recent.prefix(32))
        if let index = updated.projects.firstIndex(where: { $0.path == path }) {
            updated.projects[index].lastOpened = Date()
        }
        try save(updated)
    }

    public func upsertProject(_ record: ProjectRecord) throws {
        var updated = config
        if let index = updated.projects.firstIndex(where: { $0.path == record.path }) {
            updated.projects[index] = record
        } else {
            updated.projects.append(record)
        }
        try save(updated)
    }

    public func togglePin(path: String) throws {
        var updated = config
        let normalized = ProjectRecord.normalizePath(path)
        if let index = updated.projects.firstIndex(where: { $0.path == normalized }) {
            updated.projects[index].pinned.toggle()
        } else {
            updated.projects.append(ProjectRecord(name: URL(fileURLWithPath: normalized).lastPathComponent, path: normalized, pinned: true))
        }
        try save(updated)
    }

    public func addRoot(_ path: String) throws {
        var updated = config
        let normalized = ProjectRecord.normalizePath(path)
        guard !updated.roots.contains(normalized) else { return }
        updated.roots.append(normalized)
        try save(updated)
    }

    public func allRecordsIncludingScanned(_ scanned: [ProjectRecord]) -> [ProjectRecord] {
        ProjectIndex(records: config.projects + scanned).all
    }

    public func isManualProject(path: String) -> Bool {
        let normalized = ProjectRecord.normalizePath(path)
        return config.projects.contains { $0.path == normalized }
    }

    public func configFileURL() -> URL {
        fileURL
    }
}
