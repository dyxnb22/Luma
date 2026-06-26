import Foundation
import LumaCore

public enum NotesActionError: Error, Sendable {
    case emptyName
    case nameContainsSlash
    case alreadyExists
    case rootMissing
    case templateNotFound
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
            guard try isFolderEmpty(url) else { throw NotesDeleteError.folderNotEmpty }
        }

        try fileManager.trashItem(at: url, resultingItemURL: nil)
        await index.rebuild(after: [FSChangeEvent(path: url.path, kind: .removed)])
    }

    public func isFolderEmpty(_ url: URL) throws -> Bool {
        var isDirectory: ObjCBool = false
        guard fileManager.fileExists(atPath: url.path, isDirectory: &isDirectory), isDirectory.boolValue else {
            return true
        }
        let contents = try fileManager.contentsOfDirectory(atPath: url.path)
        return contents.filter { $0 != ".DS_Store" }.isEmpty
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

    public func findBacklinks(to target: String, limit: Int = 50) async -> [URL] {
        let trimmed = target.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty, limit > 0 else { return [] }
        guard let snapshot = await index.snapshot() else { return [] }
        let files = collectNotes(from: snapshot).map { URL(fileURLWithPath: $0.path) }
        let needle = "[[\(trimmed)]]"
        return await MarkdownContentScanner.scanFiles(containing: needle, in: files, limit: limit)
    }

    public func ensureFolder(named name: String, under parent: URL) async throws -> URL {
        let folderName = try validatedFolderName(name)
        let target = parent.appendingPathComponent(folderName, isDirectory: true)
        if !fileManager.fileExists(atPath: target.path) {
            try fileManager.createDirectory(at: target, withIntermediateDirectories: false)
            await index.rebuild(after: [FSChangeEvent(path: target.path, kind: .created)])
        }
        return target
    }

    public func createNoteInInbox(
        title: String,
        root: URL,
        inboxFolderName: String
    ) async throws -> URL {
        let inbox = try await ensureFolder(named: inboxFolderName, under: root)
        return try await createNote(name: title, inFolder: inbox)
    }

    public func openOrCreateDailyNote(
        root: URL,
        dailyFolderName: String,
        now: Date = Date()
    ) async throws -> URL {
        let dailyFolder = try await ensureFolder(named: dailyFolderName, under: root)
        let fileName = Self.dailyFileName(for: now) + ".md"
        let target = dailyFolder.appendingPathComponent(fileName)
        if !fileManager.fileExists(atPath: target.path) {
            let body = TemplateRenderer.render(NotesTemplateStore.dailyFallbackBody, title: "", now: now)
            try body.write(to: target, atomically: true, encoding: .utf8)
            await index.rebuild(after: [FSChangeEvent(path: target.path, kind: .created)])
        }
        return target
    }

    public func appendToDailyNote(
        text: String,
        root: URL,
        dailyFolderName: String,
        now: Date = Date()
    ) async throws -> URL {
        let target = try await openOrCreateDailyNote(root: root, dailyFolderName: dailyFolderName, now: now)
        let line = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !line.isEmpty else { return target }
        let existing = (try? String(contentsOf: target, encoding: .utf8)) ?? ""
        let suffix = existing.hasSuffix("\n") ? "" : "\n"
        let appended = existing + suffix + "- " + line + "\n"
        try appended.write(to: target, atomically: true, encoding: .utf8)
        await index.rebuild(after: [FSChangeEvent(path: target.path, kind: .modified)])
        return target
    }

    public func createNoteFromTemplate(
        template: NotesTemplateInfo,
        title: String,
        root: URL,
        inboxFolderName: String,
        now: Date = Date()
    ) async throws -> URL {
        let inbox = try await ensureFolder(named: inboxFolderName, under: root)
        let rendered = try NotesTemplateStore.renderTemplate(at: template.url, title: title, now: now)
        let fileName = try validatedNoteFileName(title)
        let target = inbox.appendingPathComponent(fileName)
        guard !fileManager.fileExists(atPath: target.path) else { throw NotesActionError.alreadyExists }
        try rendered.write(to: target, atomically: true, encoding: .utf8)
        await index.rebuild(after: [FSChangeEvent(path: target.path, kind: .created)])
        return target
    }

    public static func dailyFileName(for date: Date, calendar: Calendar = .current) -> String {
        TemplateRenderer.render("{{date}}", title: "", now: date, calendar: calendar)
            .trimmingCharacters(in: .whitespacesAndNewlines)
    }

    public static func weeklyReviewFileName(for date: Date = Date(), calendar: Calendar = .current) -> String {
        let week = TemplateRenderer.render("{{week}}", title: "", now: date, calendar: calendar)
            .trimmingCharacters(in: .whitespacesAndNewlines)
        return "\(week).md"
    }

    public func move(_ url: URL, toFolder folder: URL) async throws -> URL {
        let destination = folder.appendingPathComponent(url.lastPathComponent)
        guard destination.path != url.path else { return url }
        guard !fileManager.fileExists(atPath: destination.path) else { throw NotesActionError.alreadyExists }
        try fileManager.moveItem(at: url, to: destination)
        await index.rebuild(after: [
            FSChangeEvent(path: url.path, kind: .removed),
            FSChangeEvent(path: destination.path, kind: .created)
        ])
        return destination
    }

    public func createWeeklyReview(
        root: URL,
        reviewsFolderName: String,
        modifiedNotes: [NotesMeta],
        now: Date = Date(),
        calendar: Calendar = .current
    ) async throws -> URL {
        let year = String(calendar.component(.year, from: now))
        let yearFolder = try await ensureFolder(named: reviewsFolderName, under: root)
        let reviewsYear = try await ensureFolder(named: year, under: yearFolder)
        let fileName = Self.weeklyReviewFileName(for: now, calendar: calendar)
        let target = reviewsYear.appendingPathComponent(fileName)
        if fileManager.fileExists(atPath: target.path) {
            return target
        }

        var lines = [
            "# Weekly Review \(TemplateRenderer.render("{{week}}", title: "", now: now, calendar: calendar))",
            "",
            "## Notes modified this week",
            ""
        ]
        for note in modifiedNotes {
            let title = note.name
            lines.append("- [[\(title)]]")
        }
        lines.append("")
        let body = lines.joined(separator: "\n")
        try body.write(to: target, atomically: true, encoding: .utf8)
        await index.rebuild(after: [FSChangeEvent(path: target.path, kind: .created)])
        return target
    }

    public func notesInFolder(named folderName: String, root: URL) async -> [NotesNode] {
        guard let snapshot = await index.snapshot() else { return [] }
        let folderPath = root.appendingPathComponent(folderName).path
        guard let folder = findFolder(path: folderPath, in: snapshot) else { return [] }
        return collectNotes(from: folder)
    }

    private func findFolder(path: String, in node: NotesNode) -> NotesNode? {
        if node.path == path, node.kind == .folder { return node }
        for child in node.children {
            if let found = findFolder(path: path, in: child) { return found }
        }
        return nil
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
