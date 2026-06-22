import Foundation

public struct ImageReport: Sendable {
    public let orphans: [URL]
    public let brokenLinks: [(URL, String)]
    public let absolutePathLinks: [(URL, String)]

    public init(orphans: [URL], brokenLinks: [(URL, String)], absolutePathLinks: [(URL, String)]) {
        self.orphans = orphans
        self.brokenLinks = brokenLinks
        self.absolutePathLinks = absolutePathLinks
    }
}

public struct MigrationResult: Sendable {
    public let moved: Int
    public let rewritten: Int

    public init(moved: Int, rewritten: Int) {
        self.moved = moved
        self.rewritten = rewritten
    }
}

public actor NotesImageTools {
    private let root: URL
    private let fileManager: FileManager
    private let imageExtensions: Set<String> = ["png", "jpg", "jpeg", "gif", "webp", "svg", "heic"]

    public init(root: URL, fileManager: FileManager = .default) {
        self.root = root.standardizedFileURL
        self.fileManager = fileManager
    }

    public func scan() async -> ImageReport {
        let markdownFiles = collectMarkdownFiles()
        var referenced = Set<String>()
        var broken: [(URL, String)] = []
        var absolute: [(URL, String)] = []

        for note in markdownFiles {
            guard let lines = try? String(contentsOf: note, encoding: .utf8).components(separatedBy: .newlines) else { continue }
            for line in lines {
                for link in imageLinks(in: line) {
                    if link.hasPrefix("/") || link.hasPrefix("file://") {
                        absolute.append((note, link))
                    }
                    if isRemoteLink(link) { continue }
                    let resolved = resolve(link, relativeTo: note.deletingLastPathComponent())
                    if fileManager.fileExists(atPath: resolved.path) {
                        referenced.insert(resolved.standardizedFileURL.path)
                    } else {
                        broken.append((note, link))
                    }
                }
            }
        }

        let orphans = collectImageFiles().filter { url in
            !referenced.contains(url.standardizedFileURL.path)
        }
        return ImageReport(orphans: orphans, brokenLinks: broken, absolutePathLinks: absolute)
    }

    public func migrateToAssets(folderName: String = "_assets") async throws -> MigrationResult {
        let assetsFolder = root.appendingPathComponent(folderName, isDirectory: true)
        try fileManager.createDirectory(at: assetsFolder, withIntermediateDirectories: true)

        var moved = 0
        var rewritten = 0
        var pathMap: [String: String] = [:]

        for note in collectMarkdownFiles() {
            guard var content = try? String(contentsOf: note, encoding: .utf8) else { continue }
            var changed = false
            for line in content.components(separatedBy: .newlines) {
                for link in imageLinks(in: line) {
                    if isRemoteLink(link) { continue }
                    let resolved = resolve(link, relativeTo: note.deletingLastPathComponent())
                    guard fileManager.fileExists(atPath: resolved.path), imageExtensions.contains(resolved.pathExtension.lowercased()) else { continue }
                    let key = resolved.standardizedFileURL.path
                    let destinationPath: String
                    if let mapped = pathMap[key] {
                        destinationPath = mapped
                    } else {
                        let destination = uniqueDestination(for: resolved, in: assetsFolder)
                        if resolved.standardizedFileURL.path != destination.standardizedFileURL.path {
                            try fileManager.createDirectory(at: destination.deletingLastPathComponent(), withIntermediateDirectories: true)
                            if !fileManager.fileExists(atPath: destination.path) {
                                try fileManager.moveItem(at: resolved, to: destination)
                                moved += 1
                            }
                        }
                        let relative = relativePath(from: note.deletingLastPathComponent(), to: destination)
                        pathMap[key] = relative
                        destinationPath = relative
                    }
                    if link != destinationPath {
                        content = content.replacingOccurrences(of: link, with: destinationPath)
                        changed = true
                        rewritten += 1
                    }
                }
            }
            if changed {
                try content.write(to: note, atomically: true, encoding: .utf8)
            }
        }
        return MigrationResult(moved: moved, rewritten: rewritten)
    }

    public func checkTyporaConfig() async -> [String] {
        let plistURL = FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/Preferences/abnerworks.Typora.plist")
        guard let data = try? Data(contentsOf: plistURL),
              let plist = try? PropertyListSerialization.propertyList(from: data, format: nil) as? [String: Any] else {
            return ["Typora preferences file not found."]
        }

        var warnings: [String] = []
        if let copyPath = plist["copyImagePath"] as? String {
            if copyPath.isEmpty {
                warnings.append("Typora copy image path is empty.")
            } else if copyPath.hasPrefix("/") {
                warnings.append("Typora copy image path is absolute: \(copyPath)")
            }
        }
        return warnings.isEmpty ? ["Typora image path configuration looks OK."] : warnings
    }

    private func collectMarkdownFiles() -> [URL] {
        guard let enumerator = fileManager.enumerator(at: root, includingPropertiesForKeys: nil, options: [.skipsHiddenFiles]) else { return [] }
        var files: [URL] = []
        for case let url as URL in enumerator where url.pathExtension.compare("md", options: .caseInsensitive) == .orderedSame {
            files.append(url)
        }
        return files
    }

    private func collectImageFiles() -> [URL] {
        guard let enumerator = fileManager.enumerator(at: root, includingPropertiesForKeys: nil, options: [.skipsHiddenFiles]) else { return [] }
        var files: [URL] = []
        for case let url as URL in enumerator {
            if url.pathComponents.contains(where: { $0 == "_assets" }) { continue }
            guard imageExtensions.contains(url.pathExtension.lowercased()) else { continue }
            files.append(url)
        }
        return files
    }

    private func imageLinks(in line: String) -> [String] {
        var links: [String] = []
        if let markdown = try? NSRegularExpression(pattern: #"!\[[^\]]*\]\(([^)]+)\)"#) {
            let range = NSRange(line.startIndex..<line.endIndex, in: line)
            markdown.enumerateMatches(in: line, range: range) { match, _, _ in
                guard let match, match.numberOfRanges > 1, let linkRange = Range(match.range(at: 1), in: line) else { return }
                links.append(String(line[linkRange]))
            }
        }
        if let html = try? NSRegularExpression(pattern: #"<img[^>]+src="([^"]+)""#) {
            let range = NSRange(line.startIndex..<line.endIndex, in: line)
            html.enumerateMatches(in: line, range: range) { match, _, _ in
                guard let match, match.numberOfRanges > 1, let linkRange = Range(match.range(at: 1), in: line) else { return }
                links.append(String(line[linkRange]))
            }
        }
        return links
    }

    private func resolve(_ link: String, relativeTo directory: URL) -> URL {
        if link.hasPrefix("/") {
            return URL(fileURLWithPath: link)
        }
        if let url = URL(string: link), url.scheme != nil {
            return url
        }
        return directory.appendingPathComponent(link).standardizedFileURL
    }

    private func isRemoteLink(_ link: String) -> Bool {
        guard let url = URL(string: link), let scheme = url.scheme?.lowercased() else { return false }
        return scheme == "http" || scheme == "https" || scheme == "data"
    }

    private func uniqueDestination(for source: URL, in folder: URL) -> URL {
        let base = folder.appendingPathComponent(source.lastPathComponent)
        guard fileManager.fileExists(atPath: base.path) else { return base }
        var index = 1
        let stem = source.deletingPathExtension().lastPathComponent
        let ext = source.pathExtension
        while true {
            let candidate = folder.appendingPathComponent("\(stem)-\(index)").appendingPathExtension(ext)
            if !fileManager.fileExists(atPath: candidate.path) {
                return candidate
            }
            index += 1
        }
    }

    private func relativePath(from directory: URL, to file: URL) -> String {
        let from = directory.standardizedFileURL.pathComponents
        let to = file.standardizedFileURL.pathComponents
        var shared = 0
        while shared < from.count, shared < to.count, from[shared] == to[shared] {
            shared += 1
        }

        let upLevels = max(0, from.count - shared)
        let down = Array(to.dropFirst(shared))
        let components = Array(repeating: "..", count: upLevels) + down
        if components.isEmpty {
            return file.lastPathComponent
        }
        return components.joined(separator: "/")
    }
}
