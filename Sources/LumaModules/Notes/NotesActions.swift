import Foundation
import LumaServices

public enum NotesActionError: Error, Sendable {
    case emptyName
    case nameContainsSlash
    case alreadyExists
    case rootMissing
}

public enum NotesDeleteError: Error, Sendable {
    case folderNotEmpty
    case rootMissing
}

public actor NotesActions {
    private let index: NotesTreeIndex
    private let fileManager: FileManager

    public init(index: NotesTreeIndex, fileManager: FileManager = .default) {
        self.index = index
        self.fileManager = fileManager
    }

    public func createNote(name: String, inFolder folder: URL) async throws -> URL {
        let fileName = try validatedNoteFileName(name)
        let target = folder.appendingPathComponent(fileName)
        guard !fileManager.fileExists(atPath: target.path) else { throw NotesActionError.alreadyExists }
        fileManager.createFile(atPath: target.path, contents: Data(), attributes: nil)
        await index.rebuild(after: [FSChangeEvent(path: target.path, kind: .created)])
        return target
    }

    public func createFolder(name: String, inFolder folder: URL) async throws -> URL {
        let folderName = try validatedFolderName(name)
        let target = folder.appendingPathComponent(folderName, isDirectory: true)
        guard !fileManager.fileExists(atPath: target.path) else { throw NotesActionError.alreadyExists }
        try fileManager.createDirectory(at: target, withIntermediateDirectories: false)
        await index.rebuild(after: [FSChangeEvent(path: target.path, kind: .created)])
        return target
    }

    public func rename(_ url: URL, to newName: String) async throws -> URL {
        let isNote = url.pathExtension.compare("md", options: .caseInsensitive) == .orderedSame
        let destinationName = try isNote ? validatedNoteFileName(newName) : validatedFolderName(newName)
        let destination = url.deletingLastPathComponent().appendingPathComponent(destinationName)
        guard !fileManager.fileExists(atPath: destination.path) else { throw NotesActionError.alreadyExists }
        try fileManager.moveItem(at: url, to: destination)
        await index.rebuild(after: [
            FSChangeEvent(path: url.path, kind: .removed),
            FSChangeEvent(path: destination.path, kind: .created)
        ])
        return destination
    }

    public func trash(_ url: URL) async throws {
        var isDirectory: ObjCBool = false
        guard fileManager.fileExists(atPath: url.path, isDirectory: &isDirectory) else { return }

        if isDirectory.boolValue {
            let contents = try fileManager.contentsOfDirectory(atPath: url.path)
            let meaningful = contents.filter { $0 != ".DS_Store" }
            guard meaningful.isEmpty else { throw NotesDeleteError.folderNotEmpty }
        }

        try fileManager.trashItem(at: url, resultingItemURL: nil)
        await index.rebuild(after: [FSChangeEvent(path: url.path, kind: .removed)])
    }

    public func relatedNotes(in note: URL) async -> [URL] {
        guard let body = try? String(contentsOf: note, encoding: .utf8) else { return [] }
        let titles = extractWikiLinks(from: body)
        guard !titles.isEmpty else { return [] }
        guard let snapshot = await index.snapshot() else { return [] }
        let allNotes = collectNotes(from: snapshot)
        var seen = Set<String>()
        var results: [URL] = []

        for title in titles {
            let key = title.lowercased()
            guard !seen.contains(key) else { continue }
            if let match = allNotes.first(where: { $0.name.compare(title, options: .caseInsensitive) == .orderedSame }) {
                seen.insert(key)
                results.append(URL(fileURLWithPath: match.path))
            }
        }
        return results
    }

    private func collectNotes(from node: NotesNode) -> [NotesNode] {
        var results: [NotesNode] = []
        if node.kind == .note {
            results.append(node)
        }
        for child in node.children {
            results.append(contentsOf: collectNotes(from: child))
        }
        return results
    }

    private func validatedNoteFileName(_ raw: String) throws -> String {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { throw NotesActionError.emptyName }
        guard !trimmed.contains("/") else { throw NotesActionError.nameContainsSlash }
        if trimmed.lowercased().hasSuffix(".md") {
            return trimmed
        }
        return "\(trimmed).md"
    }

    private func validatedFolderName(_ raw: String) throws -> String {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { throw NotesActionError.emptyName }
        guard !trimmed.contains("/") else { throw NotesActionError.nameContainsSlash }
        return trimmed
    }

    private func extractWikiLinks(from body: String) -> [String] {
        let pattern = #"\[\[([^\]]+)\]\]"#
        guard let regex = try? NSRegularExpression(pattern: pattern) else { return [] }
        let range = NSRange(body.startIndex..<body.endIndex, in: body)
        var titles: [String] = []
        regex.enumerateMatches(in: body, range: range) { match, _, _ in
            guard let match, match.numberOfRanges > 1,
                  let titleRange = Range(match.range(at: 1), in: body) else { return }
            titles.append(String(body[titleRange]))
        }
        return titles
    }
}
