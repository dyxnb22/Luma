import Foundation
import LumaCore

public struct NotesNode: Sendable, Hashable {
    public enum Kind: Sendable { case folder, note }
    public let path: String
    public let name: String
    public let kind: Kind
    public let children: [NotesNode]

    public init(path: String, name: String, kind: Kind, children: [NotesNode]) {
        self.path = path
        self.name = name
        self.kind = kind
        self.children = children
    }
}

public actor NotesTreeIndex {
    private var rootURL: URL?
    private var tree: NotesNode?
    private let fileManager: FileManager

    public init(fileManager: FileManager = .default) {
        self.fileManager = fileManager
    }

    public func setRoot(_ root: URL?) async {
        rootURL = root?.standardizedFileURL
        tree = nil
    }

    public func warmup() async {
        guard let rootURL else {
            tree = nil
            return
        }
        tree = buildTree(at: rootURL)
    }

    public func rebuild(after events: [FSChangeEvent]) async {
        guard rootURL != nil else { return }
        if events.isEmpty {
            await warmup()
            return
        }
        let onlyModified = events.allSatisfy { $0.kind == .modified }
        if !onlyModified {
            await warmup()
        }
    }

    public func snapshot() async -> NotesNode? {
        tree
    }

    public func search(prefix query: String, limit: Int = 20) async -> [NotesNode] {
        guard let tree, !query.isEmpty else { return [] }
        let lowered = query.lowercased()
        return flatten(tree)
            .filter { $0.name.lowercased().hasPrefix(lowered) }
            .sorted { $0.name.localizedStandardCompare($1.name) == .orderedAscending }
            .prefix(limit)
            .map { $0 }
    }

    public func search(fuzzy query: String, limit: Int = 20) async -> [NotesNode] {
        guard let tree, !query.isEmpty else { return [] }
        let lowered = query.lowercased()
        let matches = flatten(tree).compactMap { node -> (NotesNode, Double)? in
            let score = FuzzyMatcher.score(query: lowered, target: node.name.lowercased())
            guard score > 0 else { return nil }
            return (node, score)
        }
        return matches
            .sorted { lhs, rhs in
                if lhs.1 != rhs.1 { return lhs.1 > rhs.1 }
                if lhs.0.name.count != rhs.0.name.count { return lhs.0.name.count < rhs.0.name.count }
                return lhs.0.name.localizedStandardCompare(rhs.0.name) == .orderedAscending
            }
            .prefix(limit)
            .map(\.0)
    }

    private func buildTree(at url: URL) -> NotesNode? {
        var isDirectory: ObjCBool = false
        guard fileManager.fileExists(atPath: url.path, isDirectory: &isDirectory), isDirectory.boolValue else {
            return nil
        }

        let children = childNodes(in: url)
        return NotesNode(
            path: url.path,
            name: url.lastPathComponent,
            kind: .folder,
            children: children
        )
    }

    private func childNodes(in directory: URL) -> [NotesNode] {
        guard let contents = try? fileManager.contentsOfDirectory(
            at: directory,
            includingPropertiesForKeys: [.isDirectoryKey],
            options: [.skipsHiddenFiles]
        ) else { return [] }

        var folders: [NotesNode] = []
        var notes: [NotesNode] = []

        for url in contents {
            let standardized = url.standardizedFileURL
            var isDirectory: ObjCBool = false
            guard fileManager.fileExists(atPath: standardized.path, isDirectory: &isDirectory) else { continue }

            if isDirectory.boolValue {
                let subtree = buildTree(at: standardized)
                if let subtree {
                    folders.append(subtree)
                }
            } else if standardized.pathExtension.compare("md", options: .caseInsensitive) == .orderedSame {
                notes.append(
                    NotesNode(
                        path: standardized.path,
                        name: standardized.deletingPathExtension().lastPathComponent,
                        kind: .note,
                        children: []
                    )
                )
            }
        }

        folders.sort { $0.name.localizedStandardCompare($1.name) == .orderedAscending }
        notes.sort { $0.name.localizedStandardCompare($1.name) == .orderedAscending }
        return folders + notes
    }

    private func flatten(_ node: NotesNode) -> [NotesNode] {
        var results: [NotesNode] = []
        if node.kind == .note {
            results.append(node)
        }
        for child in node.children {
            results.append(contentsOf: flatten(child))
        }
        return results
    }
}
