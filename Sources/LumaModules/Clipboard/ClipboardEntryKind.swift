import Foundation

public enum ClipboardEntryKind: String, Sendable, Hashable, Codable {
    case text
    case link
    case email
    case code
    case image

    public static func detect(from text: String, pasteboardTypes: [String] = []) -> ClipboardEntryKind {
        if Self.isImageTypes(pasteboardTypes) {
            return .image
        }
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return .text }

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

    public static func isImageTypes(_ types: [String]) -> Bool {
        let imagePrefixes = ["public.png", "public.tiff", "public.jpeg", "public.image", "com.apple.pict", "com.compuserve.gif"]
        return types.contains { type in
            imagePrefixes.contains { type.hasPrefix($0) || type == $0 }
        }
    }
}

public enum ClipboardListFilter: String, Sendable, CaseIterable {
    case all
    case pinned
    case image
}
