import Foundation
import LumaServices

public struct NotesHealthIssue: Sendable, Equatable {
    public enum Kind: String, Sendable {
        case frontmatter
        case brokenLink
        case duplicateName
    }

    public let kind: Kind
    public let path: String
    public let message: String

    public init(kind: Kind, path: String, message: String) {
        self.kind = kind
        self.path = path
        self.message = message
    }
}

public struct NotesHealthStats: Sendable, Equatable {
    public let noteCount: Int
    public let folderCount: Int
    public let lastWarmupMilliseconds: Double

    public init(noteCount: Int, folderCount: Int, lastWarmupMilliseconds: Double) {
        self.noteCount = noteCount
        self.folderCount = folderCount
        self.lastWarmupMilliseconds = lastWarmupMilliseconds
    }
}

public enum NotesDoctor {
    public static func diagnose(
        tree: NotesNode?,
        lastWarmupMilliseconds: Double,
        fileManager: FileManager = .default,
        issueLimit: Int = 24
    ) async -> (issues: [NotesHealthIssue], stats: NotesHealthStats) {
        guard let tree else {
            return ([], NotesHealthStats(noteCount: 0, folderCount: 0, lastWarmupMilliseconds: lastWarmupMilliseconds))
        }

        let notes = flatten(tree).filter { $0.kind == .note }
        let folders = flatten(tree).filter { $0.kind == .folder }
        var issues: [NotesHealthIssue] = []

        issues.append(contentsOf: duplicateNameIssues(from: notes, limit: issueLimit))
        if issues.count < issueLimit {
            issues.append(contentsOf: frontmatterIssues(in: notes, fileManager: fileManager, limit: issueLimit - issues.count))
        }
        if issues.count < issueLimit {
            let names = Set(notes.map { $0.name.lowercased() })
            issues.append(contentsOf: brokenLinkIssues(in: notes, knownNames: names, fileManager: fileManager, limit: issueLimit - issues.count))
        }

        let stats = NotesHealthStats(
            noteCount: notes.count,
            folderCount: max(0, folders.count - 1),
            lastWarmupMilliseconds: lastWarmupMilliseconds
        )
        return (issues, stats)
    }

    private static func duplicateNameIssues(from notes: [NotesNode], limit: Int) -> [NotesHealthIssue] {
        var byName: [String: [String]] = [:]
        for note in notes {
            let key = note.name.lowercased()
            byName[key, default: []].append(note.path)
        }
        var issues: [NotesHealthIssue] = []
        for (name, paths) in byName where paths.count > 1 {
            let listing = paths.map { URL(fileURLWithPath: $0).lastPathComponent }.joined(separator: ", ")
            issues.append(NotesHealthIssue(
                kind: .duplicateName,
                path: paths[0],
                message: "Duplicate note name “\(name)” in \(listing)"
            ))
            if issues.count >= limit { break }
        }
        return issues
    }

    private static func frontmatterIssues(
        in notes: [NotesNode],
        fileManager: FileManager,
        limit: Int
    ) -> [NotesHealthIssue] {
        var issues: [NotesHealthIssue] = []
        for note in notes {
            guard let body = try? String(contentsOf: URL(fileURLWithPath: note.path), encoding: .utf8) else { continue }
            guard body.hasPrefix("---") else { continue }
            let lines = body.split(separator: "\n", omittingEmptySubsequences: false).map(String.init)
            guard lines.count >= 2 else { continue }
            let hasClose = lines.dropFirst().prefix(100).contains { $0.trimmingCharacters(in: .whitespaces) == "---" }
            if !hasClose {
                issues.append(NotesHealthIssue(
                    kind: .frontmatter,
                    path: note.path,
                    message: "Frontmatter starts with --- but has no closing ---"
                ))
            }
            if issues.count >= limit { break }
        }
        return issues
    }

    private static func brokenLinkIssues(
        in notes: [NotesNode],
        knownNames: Set<String>,
        fileManager: FileManager,
        limit: Int
    ) -> [NotesHealthIssue] {
        var issues: [NotesHealthIssue] = []
        let pattern = #"\[\[([^\]]+)\]\]"#
        guard let regex = try? NSRegularExpression(pattern: pattern) else { return [] }

        for note in notes {
            guard let body = try? String(contentsOf: URL(fileURLWithPath: note.path), encoding: .utf8) else { continue }
            let range = NSRange(body.startIndex..<body.endIndex, in: body)
            var seen = Set<String>()
            regex.enumerateMatches(in: body, range: range) { match, _, stop in
                guard issues.count < limit else { stop.pointee = true; return }
                guard let match, match.numberOfRanges > 1,
                      let titleRange = Range(match.range(at: 1), in: body) else { return }
                let title = String(body[titleRange])
                let key = title.lowercased()
                guard !seen.contains(key) else { return }
                seen.insert(key)
                if !knownNames.contains(key) {
                    issues.append(NotesHealthIssue(
                        kind: .brokenLink,
                        path: note.path,
                        message: "Unresolved [[\(title)]]"
                    ))
                    if issues.count >= limit { stop.pointee = true }
                }
            }
            if issues.count >= limit { break }
        }
        return issues
    }

    private static func flatten(_ node: NotesNode) -> [NotesNode] {
        var results = [node]
        for child in node.children {
            results.append(contentsOf: flatten(child))
        }
        return results
    }
}
