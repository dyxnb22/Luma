import Foundation

public enum ClipboardEntryKind: String, Sendable, Hashable, Codable {
    case text
    case link
    case email
    case code
    case image
    case file
    case color

    public static func detect(
        from text: String,
        pasteboardTypes: [String] = [],
        fileURLs: [String]? = nil
    ) -> ClipboardEntryKind {
        if Self.isFileTypes(pasteboardTypes) || (fileURLs?.isEmpty == false) {
            return .file
        }
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        if Self.isTextTypes(pasteboardTypes), !trimmed.isEmpty {
            if Self.looksLikeColor(trimmed) {
                return .color
            }
            if Self.looksLikeEmail(trimmed) {
                return .email
            }
            if Self.looksLikeURL(trimmed) {
                return .link
            }
            if Self.looksLikeCode(trimmed) {
                return .code
            }
            return .text
        }
        if Self.isImageTypes(pasteboardTypes) {
            return .image
        }
        guard !trimmed.isEmpty else { return .text }

        if Self.looksLikeColor(trimmed) {
            return .color
        }
        if Self.looksLikeEmail(trimmed) {
            return .email
        }
        if Self.looksLikeURL(trimmed) {
            return .link
        }
        if Self.looksLikeCode(trimmed) {
            return .code
        }
        return .text
    }

    private static func looksLikeEmail(_ text: String) -> Bool {
        guard !text.contains(where: \.isWhitespace), text.contains("@") else { return false }
        let parts = text.split(separator: "@", omittingEmptySubsequences: false)
        guard parts.count == 2, parts[0].isEmpty == false, parts[1].contains(".") else { return false }
        return true
    }

    private static func looksLikeURL(_ text: String) -> Bool {
        let lower = text.lowercased()
        if lower.hasPrefix("http://") || lower.hasPrefix("https://") { return true }
        if lower.hasPrefix("www.") && lower.contains(".") { return true }
        return false
    }

    private static func looksLikeCode(_ text: String) -> Bool {
        let lines = text.split(separator: "\n", omittingEmptySubsequences: false)
        guard lines.count >= 2 else { return false }
        let indicators = ["{", "}", ";", "func ", "def ", "class ", "import ", "const ", "let ", "var "]
        return indicators.contains { text.contains($0) }
    }

    public static func looksLikeColor(_ text: String) -> Bool {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.range(of: #"^#([0-9A-Fa-f]{3}|[0-9A-Fa-f]{6})$"#, options: .regularExpression) != nil {
            return true
        }
        if trimmed.range(of: #"^rgb\(\s*\d{1,3}\s*,\s*\d{1,3}\s*,\s*\d{1,3}\s*\)$"#, options: .regularExpression) != nil {
            return true
        }
        if trimmed.range(of: #"^hsl\(\s*\d{1,3}(\.\d+)?\s*,\s*\d{1,3}(\.\d+)?%\s*,\s*\d{1,3}(\.\d+)?%\s*\)$"#, options: .regularExpression) != nil {
            return true
        }
        return false
    }

    public static func normalizedColorHex(from text: String) -> String? {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard looksLikeColor(trimmed) else { return nil }
        if trimmed.hasPrefix("#") {
            let hex = String(trimmed.dropFirst())
            if hex.count == 3 {
                let expanded = hex.map { String($0) + String($0) }.joined()
                return "#\(expanded.uppercased())"
            }
            return "#\(hex.uppercased())"
        }
        return trimmed.lowercased()
    }

    public static func isImageTypes(_ types: [String]) -> Bool {
        let imagePrefixes = ["public.png", "public.tiff", "public.jpeg", "public.image", "com.apple.pict", "com.compuserve.gif"]
        return types.contains { type in
            imagePrefixes.contains { type.hasPrefix($0) || type == $0 }
        }
    }

    public static func isTextTypes(_ types: [String]) -> Bool {
        let textTypes: Set<String> = [
            "public.utf8-plain-text",
            "NSStringPboardType",
            "public.plain-text",
            "public.text",
            "public.html",
            "public.rtf"
        ]
        return types.contains { type in
            textTypes.contains(type) || type.hasPrefix("public.text")
        }
    }

    public static func isFileTypes(_ types: [String]) -> Bool {
        types.contains { $0 == "public.file-url" || $0 == "NSURLPboardType" }
    }
}

public enum ClipboardListFilter: String, Sendable, CaseIterable {
    case all
    case pinned
    case image
}

public enum ClipboardPasteBehavior: String, Sendable, Codable, CaseIterable {
    case pasteDirectly
    case copyOnly

    public var displayName: String {
        switch self {
        case .pasteDirectly: return "Paste directly"
        case .copyOnly: return "Copy only"
        }
    }
}

public enum ClipboardRecentClearWindow: String, Sendable, CaseIterable {
    case last5Minutes
    case lastHour
    case today

    public var displayName: String {
        switch self {
        case .last5Minutes: return "Last 5 minutes"
        case .lastHour: return "Last hour"
        case .today: return "Today"
        }
    }

    public func cutoff(from now: Date, calendar: Calendar = .current) -> Date {
        switch self {
        case .last5Minutes:
            return now.addingTimeInterval(-5 * 60)
        case .lastHour:
            return now.addingTimeInterval(-3600)
        case .today:
            return calendar.startOfDay(for: now)
        }
    }
}
