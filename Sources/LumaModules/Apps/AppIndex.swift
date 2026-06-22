import Foundation

public struct AppRecord: Sendable, Hashable, Codable {
    public let name: String
    public let bundleID: String
    public let url: URL

    public init(name: String, bundleID: String, url: URL) {
        self.name = name
        self.bundleID = bundleID
        self.url = url
    }
}

public struct AppIndex: Sendable {
    private let apps: [AppRecord]

    public init(apps: [AppRecord]) {
        self.apps = apps.sorted { $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending }
    }

    public func search(_ query: String, limit: Int = 20) -> [AppRecord] {
        let normalized = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !normalized.isEmpty else { return Array(apps.prefix(limit)) }

        return apps
            .map { app in (app, score(app: app, query: normalized)) }
            .filter { $0.1 > 0 }
            .sorted { lhs, rhs in
                if lhs.1 == rhs.1 { return lhs.0.name < rhs.0.name }
                return lhs.1 > rhs.1
            }
            .prefix(limit)
            .map(\.0)
    }

    private func score(app: AppRecord, query: String) -> Int {
        let name = app.name.lowercased()
        let bundle = app.bundleID.lowercased()
        if name == query { return 100 }
        if name.hasPrefix(query) { return 80 }
        if name.contains(query) { return 60 }
        if bundle.contains(query) { return 30 }
        return 0
    }
}

public enum AppScanner {
    public static func scan(fileManager: FileManager = .default) -> [AppRecord] {
        let roots = [
            URL(fileURLWithPath: "/Applications"),
            URL(fileURLWithPath: "/System/Applications"),
            fileManager.homeDirectoryForCurrentUser.appendingPathComponent("Applications")
        ]

        var seen: Set<URL> = []
        var records: [AppRecord] = []
        for root in roots where fileManager.fileExists(atPath: root.path) {
            guard let enumerator = fileManager.enumerator(
                at: root,
                includingPropertiesForKeys: [.isDirectoryKey],
                options: [.skipsHiddenFiles, .skipsPackageDescendants]
            ) else { continue }

            for case let url as URL in enumerator where url.pathExtension == "app" {
                guard seen.insert(url).inserted else { continue }
                records.append(AppRecord(
                    name: url.deletingPathExtension().lastPathComponent,
                    bundleID: Bundle(url: url)?.bundleIdentifier ?? url.lastPathComponent,
                    url: url
                ))
            }
        }
        return records
    }
}
