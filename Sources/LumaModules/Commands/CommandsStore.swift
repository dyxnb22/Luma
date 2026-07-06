import Foundation
import LumaCore

public actor CommandsStore {
    private let fileURL: URL
    private let fileManager: FileManager
    private var config: CommandsConfig

    public init(fileURL: URL = CommandsStore.defaultFileURL, fileManager: FileManager = .default) {
        self.fileURL = fileURL
        self.fileManager = fileManager
        let result = JSONConfigPersistence.load(from: fileURL, fallback: CommandsConfig.empty, fileManager: fileManager)
        self.config = result.value
        self.lastLoadWasCorrupt = result.wasCorrupt
    }

    public static var defaultFileURL: URL {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support")
        return base.appendingPathComponent("Luma/commands.json", isDirectory: false)
    }

    public func current() -> CommandsConfig {
        config
    }

    public func reload() {
        let result = JSONConfigPersistence.load(from: fileURL, fallback: CommandsConfig.empty, fileManager: fileManager)
        lastLoadWasCorrupt = result.wasCorrupt
        config = result.value
    }

    public func configFileURL() -> URL {
        fileURL
    }

    private var lastLoadWasCorrupt = false

    public func loadWasCorrupt() -> Bool {
        lastLoadWasCorrupt
    }
}
