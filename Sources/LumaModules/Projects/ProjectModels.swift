import Foundation

public enum ProjectOpener: String, Sendable, Codable, CaseIterable {
    case cursor
    case vscode
    case finder
    case terminal
}

public struct ProjectRecord: Sendable, Codable, Hashable {
    public var name: String
    public var path: String
    public var aliases: [String]
    public var preferredOpener: ProjectOpener
    public var pinned: Bool
    public var lastOpened: Date?

    public init(
        name: String,
        path: String,
        aliases: [String] = [],
        preferredOpener: ProjectOpener = .cursor,
        pinned: Bool = false,
        lastOpened: Date? = nil
    ) {
        self.name = name
        self.path = path
        self.aliases = aliases
        self.preferredOpener = preferredOpener
        self.pinned = pinned
        self.lastOpened = lastOpened
    }

    public func normalized(home: URL = FileManager.default.homeDirectoryForCurrentUser) -> ProjectRecord {
        ProjectRecord(
            name: name,
            path: Self.normalizePath(path, home: home),
            aliases: aliases,
            preferredOpener: preferredOpener,
            pinned: pinned,
            lastOpened: lastOpened
        )
    }

    public static func normalizePath(_ path: String, home: URL = FileManager.default.homeDirectoryForCurrentUser) -> String {
        if path == "~" {
            return home.path
        }
        if path.hasPrefix("~/") {
            return home.appendingPathComponent(String(path.dropFirst(2))).standardizedFileURL.path
        }
        return URL(fileURLWithPath: (path as NSString).expandingTildeInPath).standardizedFileURL.path
    }
}

public struct ProjectsConfig: Codable, Sendable, Equatable {
    public var roots: [String]
    public var projects: [ProjectRecord]
    public var recent: [String]

    public static let empty = ProjectsConfig(roots: [], projects: [], recent: [])

    public init(roots: [String], projects: [ProjectRecord], recent: [String]) {
        self.roots = roots
        self.projects = projects
        self.recent = recent
    }
}
