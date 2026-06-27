import Foundation

public struct Quicklink: Codable, Sendable, Hashable, Identifiable {
    public var id: UUID
    public var name: String
    public var trigger: String
    public var urlTemplate: String
    public var openWith: String?
    public var icon: String?

    public init(
        id: UUID = UUID(),
        name: String,
        trigger: String,
        urlTemplate: String,
        openWith: String? = nil,
        icon: String? = nil
    ) {
        self.id = id
        self.name = name
        self.trigger = trigger
        self.urlTemplate = urlTemplate
        self.openWith = openWith
        self.icon = icon
    }
}

public struct QuicklinkExpansion: Sendable, Hashable {
    public let quicklink: Quicklink
    public let query: String
    public let urlString: String
    public let url: URL

    public init(quicklink: Quicklink, query: String, urlString: String, url: URL) {
        self.quicklink = quicklink
        self.query = query
        self.urlString = urlString
        self.url = url
    }
}
