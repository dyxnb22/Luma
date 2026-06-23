import Foundation

public struct AppRecord: Sendable, Hashable, Codable {
    public let bundleID: String
    public let url: URL
    public let name: String
    public let localizedName: String
    public let aliases: [String]
    public let pinyinFull: String
    public let pinyinInitials: String

    public init(
        bundleID: String,
        url: URL,
        name: String,
        localizedName: String = "",
        aliases: [String] = [],
        pinyinFull: String = "",
        pinyinInitials: String = ""
    ) {
        self.bundleID = bundleID
        self.url = url
        self.name = name
        self.localizedName = localizedName.isEmpty ? name : localizedName
        self.aliases = aliases
        self.pinyinFull = pinyinFull
        self.pinyinInitials = pinyinInitials
    }

    private enum CodingKeys: String, CodingKey {
        case bundleID, url, name, localizedName, aliases, pinyinFull, pinyinInitials
    }

    /// Backward-compatible decode for v1 cache entries missing new fields.
    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        bundleID = try container.decode(String.self, forKey: .bundleID)
        url = try container.decode(URL.self, forKey: .url)
        name = try container.decode(String.self, forKey: .name)
        localizedName = try container.decodeIfPresent(String.self, forKey: .localizedName) ?? name
        aliases = try container.decodeIfPresent([String].self, forKey: .aliases) ?? []
        pinyinFull = try container.decodeIfPresent(String.self, forKey: .pinyinFull) ?? ""
        pinyinInitials = try container.decodeIfPresent(String.self, forKey: .pinyinInitials) ?? ""
    }

    public var displayTitle: String {
        if !localizedName.isEmpty { return localizedName }
        return name
    }

    public var subtitlePath: String {
        url.path
    }
}

public struct AppIndex: Sendable {
    private let apps: [AppRecord]

    public init(apps: [AppRecord]) {
        self.apps = apps.sorted { $0.displayTitle.localizedCaseInsensitiveCompare($1.displayTitle) == .orderedAscending }
    }

    public func search(_ query: String, limit: Int = 20) -> [AppRecord] {
        let normalized = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !normalized.isEmpty else { return Array(apps.prefix(limit)) }

        return apps
            .map { app in (app, score(app: app, query: normalized)) }
            .filter { $0.1 > 0 }
            .sorted { lhs, rhs in
                if lhs.1 == rhs.1 { return lhs.0.displayTitle < rhs.0.displayTitle }
                return lhs.1 > rhs.1
            }
            .prefix(limit)
            .map(\.0)
    }

    private func score(app: AppRecord, query: String) -> Int {
        let q = query.lowercased()
        let qCompact = q.filter { !$0.isWhitespace }

        if app.name.lowercased() == q { return 1000 }
        if app.localizedName.lowercased() == q { return 990 }
        if app.aliases.contains(where: { $0.lowercased() == q }) { return 980 }

        let allNames = [app.name, app.localizedName] + app.aliases
        if allNames.contains(where: { $0.lowercased().hasPrefix(q) }) { return 800 }
        if app.pinyinFull.hasPrefix(q) { return 780 }
        if app.pinyinInitials.hasPrefix(qCompact) { return 760 }

        if allNames.contains(where: { $0.lowercased().contains(q) }) { return 600 }
        if app.pinyinFull.contains(q) { return 580 }
        if app.bundleID.lowercased().contains(q) { return 300 }

        if isSubsequence(qCompact, of: app.pinyinInitials) { return 400 }
        for name in allNames {
            if isSubsequence(qCompact, of: name.lowercased()) { return 350 }
        }
        return 0
    }

    private func isSubsequence(_ needle: String, of haystack: String) -> Bool {
        guard !needle.isEmpty else { return false }
        var idx = haystack.startIndex
        for ch in needle {
            guard let found = haystack[idx...].firstIndex(of: ch) else { return false }
            idx = haystack.index(after: found)
        }
        return true
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
                guard let bundle = Bundle(url: url) else { continue }
                let bundleID = bundle.bundleIdentifier ?? url.lastPathComponent
                let stem = url.deletingPathExtension().lastPathComponent
                let displayName = bundle.localizedInfoDictionary?["CFBundleDisplayName"] as? String
                    ?? bundle.infoDictionary?["CFBundleDisplayName"] as? String
                    ?? bundle.localizedInfoDictionary?["CFBundleName"] as? String
                    ?? bundle.infoDictionary?["CFBundleName"] as? String
                    ?? stem
                let zhNames = readZhInfoPlistStrings(in: bundle)
                let alias = AppAliasTable.aliases(forBundleID: bundleID, name: stem, zhNames: zhNames)
                let allText = [stem, displayName] + zhNames + alias
                let pinyinFull = PinyinIndex.full(from: allText)
                let pinyinInitials = PinyinIndex.initials(from: allText)
                records.append(AppRecord(
                    bundleID: bundleID,
                    url: url,
                    name: stem,
                    localizedName: displayName,
                    aliases: Array(Set(alias + zhNames)),
                    pinyinFull: pinyinFull,
                    pinyinInitials: pinyinInitials
                ))
            }
        }
        return records
    }

    private static func readZhInfoPlistStrings(in bundle: Bundle) -> [String] {
        let candidates = ["zh-Hans", "zh-Hant", "zh_CN", "Base"]
        var results: [String] = []
        for lang in candidates {
            guard let url = bundle.url(forResource: "InfoPlist", withExtension: "strings", subdirectory: nil, localization: lang) else { continue }
            guard let dict = NSDictionary(contentsOf: url) as? [String: String] else { continue }
            if let name = dict["CFBundleDisplayName"] ?? dict["CFBundleName"] {
                results.append(name)
            }
        }
        return results
    }
}
