import Foundation

public actor NotesMetaIndex {
    private var entries: [String: NotesMeta] = [:]
    private let fileManager: FileManager

    public init(fileManager: FileManager = .default) {
        self.fileManager = fileManager
    }

    public func rebuild(from tree: NotesNode?) {
        guard let tree else {
            entries = [:]
            return
        }
        var next: [String: NotesMeta] = [:]
        for node in flatten(tree) {
            next[node.path] = loadMeta(for: node)
        }
        entries = next
    }

    public func update(path: String, in tree: NotesNode?) {
        guard let tree, let node = flatten(tree).first(where: { $0.path == path }) else {
            entries.removeValue(forKey: path)
            return
        }
        entries[path] = loadMeta(for: node)
    }

    public func remove(path: String) {
        entries.removeValue(forKey: path)
    }

    public func meta(for path: String) -> NotesMeta? {
        entries[path]
    }

    public func allEntries() -> [NotesMeta] {
        Array(entries.values)
    }

    public func pinnedNotes() -> [NotesMeta] {
        entries.values.filter(\.pinned).sorted(by: sortByMtimeDesc)
    }

    public func notes(withTag tag: String) -> [NotesMeta] {
        let key = tag.lowercased()
        return entries.values.filter { $0.tags.contains { $0.compare(key, options: .caseInsensitive) == .orderedSame } }
            .sorted(by: sortByName)
    }

    public func notes(withType type: String) -> [NotesMeta] {
        let key = type.lowercased()
        return entries.values.filter { $0.type?.compare(key, options: .caseInsensitive) == .orderedSame }
            .sorted(by: sortByName)
    }

    public func modifiedSince(_ date: Date) -> [NotesMeta] {
        entries.values.filter { $0.mtime >= date }.sorted(by: sortByMtimeDesc)
    }

    public func search(filter: NotesMetaFilter, limit: Int = 20) -> [NotesMeta] {
        var results = Array(entries.values)
        if let tag = filter.tag?.lowercased(), !tag.isEmpty {
            results = results.filter { meta in
                meta.tags.contains { $0.lowercased() == tag }
            }
        }
        if let type = filter.type?.lowercased(), !type.isEmpty {
            results = results.filter { $0.type?.lowercased() == type }
        }
        if let text = filter.text?.trimmingCharacters(in: .whitespacesAndNewlines), !text.isEmpty {
            let lowered = text.lowercased()
            results = results.filter { $0.name.lowercased().contains(lowered) }
        }
        return Array(results.sorted(by: sortByMtimeDesc).prefix(limit))
    }

    public func distinctTypes() -> [String] {
        let types = entries.values.compactMap(\.type).filter { !$0.isEmpty }
        return Array(Set(types)).sorted { $0.localizedStandardCompare($1) == .orderedAscending }
    }

    public func distinctTags() -> [String] {
        let tags = entries.values.flatMap(\.tags).filter { !$0.isEmpty }
        return Array(Set(tags)).sorted { $0.localizedStandardCompare($1) == .orderedAscending }
    }

    private static let frontmatterReadCap = 4096

    private func loadMeta(for node: NotesNode) -> NotesMeta {
        let url = URL(fileURLWithPath: node.path)
        let mtime = (try? url.resourceValues(forKeys: [.contentModificationDateKey]).contentModificationDate) ?? .distantPast
        let body = readFilePrefix(url: url)
        let fields = FrontmatterParser.parse(body)
        return NotesMeta(
            path: node.path,
            name: node.name,
            tags: fields.tags,
            type: fields.type,
            pinned: fields.pinned,
            mtime: mtime
        )
    }

    private func readFilePrefix(url: URL) -> String {
        guard let handle = try? FileHandle(forReadingFrom: url) else { return "" }
        defer { try? handle.close() }
        guard let data = try? handle.read(upToCount: Self.frontmatterReadCap) else { return "" }
        return String(data: data, encoding: .utf8) ?? ""
    }

    private func flatten(_ node: NotesNode) -> [NotesNode] {
        var results: [NotesNode] = []
        if node.kind == .note { results.append(node) }
        for child in node.children {
            results.append(contentsOf: flatten(child))
        }
        return results
    }

    private func sortByMtimeDesc(_ lhs: NotesMeta, _ rhs: NotesMeta) -> Bool {
        if lhs.mtime != rhs.mtime { return lhs.mtime > rhs.mtime }
        return lhs.name.localizedStandardCompare(rhs.name) == .orderedAscending
    }

    private func sortByName(_ lhs: NotesMeta, _ rhs: NotesMeta) -> Bool {
        lhs.name.localizedStandardCompare(rhs.name) == .orderedAscending
    }
}
