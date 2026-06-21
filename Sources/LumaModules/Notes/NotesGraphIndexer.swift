import Foundation

public struct NoteNode: Sendable, Hashable {
    public let path: String
    public let title: String
    public let tags: Set<String>
}

public struct NoteEdge: Sendable, Hashable {
    public let from: String
    public let to: String
    public let kind: String
}

public struct NotesGraph: Sendable, Hashable {
    public let nodes: [NoteNode]
    public let edges: [NoteEdge]
}

public enum NotesGraphIndexer {
    public static func index(files: [String: String]) -> NotesGraph {
        var nodes: [NoteNode] = []
        var edges: [NoteEdge] = []

        for (path, body) in files {
            let title = URL(fileURLWithPath: path).deletingPathExtension().lastPathComponent
            let tags = Set(matches(in: body, pattern: #"(?<!\w)#([A-Za-z0-9_\-/]+)"#))
            nodes.append(NoteNode(path: path, title: title, tags: tags))

            for target in matches(in: body, pattern: #"\[\[([^\]]+)\]\]"#) {
                edges.append(NoteEdge(from: path, to: target, kind: "wiki"))
            }
            for tag in tags {
                edges.append(NoteEdge(from: path, to: tag, kind: "tag"))
            }
        }

        return NotesGraph(nodes: nodes.sorted { $0.title < $1.title }, edges: edges)
    }

    private static func matches(in text: String, pattern: String) -> [String] {
        guard let regex = try? NSRegularExpression(pattern: pattern) else { return [] }
        let range = NSRange(text.startIndex..<text.endIndex, in: text)
        return regex.matches(in: text, range: range).compactMap { match in
            guard match.numberOfRanges > 1, let swiftRange = Range(match.range(at: 1), in: text) else { return nil }
            return String(text[swiftRange])
        }
    }
}
