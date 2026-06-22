import Foundation

public struct NotesRootConfig: Codable, Sendable, Equatable {
    public var root: URL?
    public var expandedFolders: Set<String>
    public var recent: [String]

    public static let empty = NotesRootConfig(root: nil, expandedFolders: [], recent: [])

    public init(root: URL?, expandedFolders: Set<String>, recent: [String] = []) {
        self.root = root
        self.expandedFolders = expandedFolders
        self.recent = recent
    }

    private enum CodingKeys: String, CodingKey {
        case root
        case expandedFolders
        case recent
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        if let rootString = try container.decodeIfPresent(String.self, forKey: .root) {
            root = URL(fileURLWithPath: rootString)
        } else {
            root = nil
        }
        let folders = try container.decodeIfPresent([String].self, forKey: .expandedFolders) ?? []
        expandedFolders = Set(folders)
        recent = try container.decodeIfPresent([String].self, forKey: .recent) ?? []
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encodeIfPresent(root?.path, forKey: .root)
        try container.encode(Array(expandedFolders).sorted(), forKey: .expandedFolders)
        try container.encode(recent, forKey: .recent)
    }
}

public actor NotesRootConfigStore {
    private let fileURL: URL
    private let fileManager: FileManager

    public init(fileURL: URL = NotesRootConfigStore.defaultFileURL, fileManager: FileManager = .default) {
        self.fileURL = fileURL
        self.fileManager = fileManager
    }

    public static var defaultFileURL: URL {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support")
        return base.appendingPathComponent("Luma/notes.json", isDirectory: false)
    }

    public func load() -> NotesRootConfig {
        guard let data = try? Data(contentsOf: fileURL) else { return .empty }
        guard let config = try? JSONDecoder().decode(NotesRootConfig.self, from: data) else { return .empty }
        return config
    }

    public func save(_ config: NotesRootConfig) throws {
        let directory = fileURL.deletingLastPathComponent()
        try fileManager.createDirectory(at: directory, withIntermediateDirectories: true)
        let data = try JSONEncoder().encode(config)
        try data.write(to: fileURL, options: .atomic)
    }
}
