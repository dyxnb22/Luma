import Foundation

public struct URLQuicklinkDraft: Sendable, Equatable, Codable, Hashable {
    public var name: String
    public var trigger: String
    public var urlTemplate: String

    public init(name: String, trigger: String, urlTemplate: String) {
        self.name = name
        self.trigger = trigger
        self.urlTemplate = urlTemplate
    }

    public static func from(url: URL) -> URLQuicklinkDraft {
        let host = url.host?.replacingOccurrences(of: "www.", with: "") ?? "link"
        let slug = host.split(separator: ".").first.map(String.init) ?? "link"
        let trigger = String(slug.prefix(12)).lowercased().filter { $0.isLetter || $0.isNumber }
        return URLQuicklinkDraft(
            name: host,
            trigger: trigger.isEmpty ? "link" : trigger,
            urlTemplate: url.absoluteString
        )
    }
}

public enum URLTextParser {
    public static func firstHTTPURL(in text: String) -> URL? {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return nil }
        if let url = URL(string: trimmed), isWebURL(url) {
            return url
        }
        for word in trimmed.split(whereSeparator: \.isWhitespace) {
            let token = String(word).trimmingCharacters(in: CharacterSet(charactersIn: "<>\"'"))
            if let url = URL(string: token), isWebURL(url) {
                return url
            }
        }
        return nil
    }

    private static func isWebURL(_ url: URL) -> Bool {
        guard let scheme = url.scheme?.lowercased() else { return false }
        return scheme == "http" || scheme == "https"
    }
}
