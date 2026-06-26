import Foundation

public actor CommandsStore {
    private let fileURL: URL
    private let fileManager: FileManager
    private var config: CommandsConfig

    public init(fileURL: URL = CommandsStore.defaultFileURL, fileManager: FileManager = .default) {
        self.fileURL = fileURL
        self.fileManager = fileManager
        self.config = Self.load(from: fileURL)
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
        config = Self.load(from: fileURL)
    }

    public func configFileURL() -> URL {
        fileURL
    }

    private static func load(from url: URL) -> CommandsConfig {
        guard let data = try? Data(contentsOf: url) else { return .empty }
        guard let config = try? JSONDecoder().decode(CommandsConfig.self, from: data) else { return .empty }
        return config
    }
}
