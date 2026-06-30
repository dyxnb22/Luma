import Foundation
import LumaCore

public protocol QuicklinkDraftSource: Sendable {
    func quicklinkDraft() -> URLQuicklinkDraft?
}

public struct URLQuicklinkDraftSource: QuicklinkDraftSource {
    public let url: URL

    public init(url: URL) {
        self.url = url
    }

    public func quicklinkDraft() -> URLQuicklinkDraft? {
        URLQuicklinkDraft.from(url: url)
    }
}

public struct TextQuicklinkDraftSource: QuicklinkDraftSource {
    public let text: String

    public init(text: String) {
        self.text = text
    }

    public func quicklinkDraft() -> URLQuicklinkDraft? {
        guard let url = URLTextParser.firstHTTPURL(in: text) else { return nil }
        return URLQuicklinkDraftSource(url: url).quicklinkDraft()
    }
}

public struct ProjectQuicklinkDraftSource: QuicklinkDraftSource {
    public let context: CurrentProjectContext

    public init(context: CurrentProjectContext) {
        self.context = context
    }

    public func quicklinkDraft() -> URLQuicklinkDraft? {
        guard let path = context.matchedProjectPath else { return nil }
        let name = context.projectName ?? context.projectLabel
        let slug = ProjectContextSuggestions.projectSlug(for: context)
        return URLQuicklinkDraft(
            name: "\(name) folder",
            trigger: slug,
            urlTemplate: URL(fileURLWithPath: path).absoluteString
        )
    }
}
