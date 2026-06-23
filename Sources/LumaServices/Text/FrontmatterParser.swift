import Foundation

public struct FrontmatterFields: Sendable, Equatable {
    public let title: String?
    public let tags: [String]
    public let type: String?
    public let pinned: Bool

    public static let empty = FrontmatterFields(title: nil, tags: [], type: nil, pinned: false)

    public init(title: String?, tags: [String], type: String?, pinned: Bool) {
        self.title = title
        self.tags = tags
        self.type = type
        self.pinned = pinned
    }
}

public enum FrontmatterParser {
    public static func parse(_ markdown: String) -> FrontmatterFields {
        guard markdown.hasPrefix("---") else { return .empty }
        let lines = markdown.split(separator: "\n", omittingEmptySubsequences: false).map(String.init)
        guard lines.count >= 2 else { return .empty }

        var endIndex: Int?
        for index in 1..<lines.count {
            if lines[index].trimmingCharacters(in: .whitespaces) == "---" {
                endIndex = index
                break
            }
        }
        guard let endIndex else { return .empty }

        let body = lines[1..<endIndex]
        var title: String?
        var tags: [String] = []
        var type: String?
        var pinned = false

        for line in body {
            let trimmed = line.trimmingCharacters(in: .whitespaces)
            if trimmed.isEmpty || trimmed.hasPrefix("#") { continue }
            guard let colon = trimmed.firstIndex(of: ":") else { continue }
            let key = String(trimmed[..<colon]).trimmingCharacters(in: .whitespaces).lowercased()
            let rawValue = String(trimmed[trimmed.index(after: colon)...]).trimmingCharacters(in: .whitespaces)
            switch key {
            case "title":
                title = unquote(rawValue)
            case "type":
                type = unquote(rawValue)
            case "tags":
                tags = parseTags(rawValue)
            case "pinned":
                pinned = parseBool(rawValue)
            default:
                continue
            }
        }

        return FrontmatterFields(title: title, tags: tags, type: type, pinned: pinned)
    }

    private static func unquote(_ value: String) -> String {
        let trimmed = value.trimmingCharacters(in: .whitespaces)
        if (trimmed.hasPrefix("\"") && trimmed.hasSuffix("\"")) || (trimmed.hasPrefix("'") && trimmed.hasSuffix("'")) {
            return String(trimmed.dropFirst().dropLast())
        }
        return trimmed
    }

    private static func parseTags(_ value: String) -> [String] {
        let trimmed = value.trimmingCharacters(in: .whitespaces)
        if trimmed.hasPrefix("[") && trimmed.hasSuffix("]") {
            let inner = String(trimmed.dropFirst().dropLast())
            return inner.split(separator: ",").map { unquote(String($0).trimmingCharacters(in: .whitespaces)) }.filter { !$0.isEmpty }
        }
        let single = unquote(trimmed)
        return single.isEmpty ? [] : [single]
    }

    private static func parseBool(_ value: String) -> Bool {
        switch value.lowercased() {
        case "true", "yes", "1": return true
        default: return false
        }
    }
}
