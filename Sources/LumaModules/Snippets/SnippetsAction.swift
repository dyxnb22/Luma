import Foundation

public enum SnippetsAction: Codable, Sendable {
    case copy(id: UUID)
    case paste(id: UUID)
    case create(title: String)
    case prepareDraft(SnippetDraft)
}

public struct SnippetDraft: Codable, Sendable, Equatable, Hashable {
    public var title: String
    public var trigger: String
    public var content: String
    public var tags: [String]

    public init(title: String, trigger: String = "", content: String, tags: [String] = []) {
        self.title = title
        self.trigger = trigger
        self.content = content
        self.tags = tags
    }
}
