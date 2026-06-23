import Foundation

public struct NotesMeta: Sendable, Equatable {
    public let path: String
    public let name: String
    public let tags: [String]
    public let type: String?
    public let pinned: Bool
    public let mtime: Date

    public init(path: String, name: String, tags: [String], type: String?, pinned: Bool, mtime: Date) {
        self.path = path
        self.name = name
        self.tags = tags
        self.type = type
        self.pinned = pinned
        self.mtime = mtime
    }
}

public struct NotesMetaFilter: Sendable, Equatable {
    public let tag: String?
    public let type: String?
    public let text: String?

    public init(tag: String? = nil, type: String? = nil, text: String? = nil) {
        self.tag = tag
        self.type = type
        self.text = text
    }
}
