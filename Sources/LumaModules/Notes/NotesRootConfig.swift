import Foundation

public struct NotesRootConfig: Codable, Sendable, Equatable {
    public static let schemaVersion = 1

    public var root: URL?
    public var expandedFolders: Set<String>
    public var recent: [String]
    public var inboxFolderName: String
    public var dailyFolderName: String
    public var templatesFolderName: String
    public var reviewsFolderName: String

    public static let empty = NotesRootConfig(
        root: nil,
        expandedFolders: [],
        recent: [],
        inboxFolderName: "Inbox",
        dailyFolderName: "Daily",
        templatesFolderName: "_templates",
        reviewsFolderName: "Reviews"
    )

    public init(
        root: URL?,
        expandedFolders: Set<String>,
        recent: [String] = [],
        inboxFolderName: String = "Inbox",
        dailyFolderName: String = "Daily",
        templatesFolderName: String = "_templates",
        reviewsFolderName: String = "Reviews"
    ) {
        self.root = root
        self.expandedFolders = expandedFolders
        self.recent = recent
        self.inboxFolderName = inboxFolderName
        self.dailyFolderName = dailyFolderName
        self.templatesFolderName = templatesFolderName
        self.reviewsFolderName = reviewsFolderName
    }

    private enum CodingKeys: String, CodingKey {
        case root
        case expandedFolders
        case recent
        case inboxFolderName
        case dailyFolderName
        case templatesFolderName
        case reviewsFolderName
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
        inboxFolderName = try container.decodeIfPresent(String.self, forKey: .inboxFolderName) ?? "Inbox"
        dailyFolderName = try container.decodeIfPresent(String.self, forKey: .dailyFolderName) ?? "Daily"
        templatesFolderName = try container.decodeIfPresent(String.self, forKey: .templatesFolderName) ?? "_templates"
        reviewsFolderName = try container.decodeIfPresent(String.self, forKey: .reviewsFolderName) ?? "Reviews"
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encodeIfPresent(root?.path, forKey: .root)
        try container.encode(Array(expandedFolders).sorted(), forKey: .expandedFolders)
        try container.encode(recent, forKey: .recent)
        try container.encode(inboxFolderName, forKey: .inboxFolderName)
        try container.encode(dailyFolderName, forKey: .dailyFolderName)
        try container.encode(templatesFolderName, forKey: .templatesFolderName)
        try container.encode(reviewsFolderName, forKey: .reviewsFolderName)
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
