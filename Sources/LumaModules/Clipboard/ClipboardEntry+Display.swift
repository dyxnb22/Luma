import Foundation

extension ClipboardEntry {
    public var metadataLine: String {
        let ago = RelativeDateTimeFormatter().localizedString(for: createdAt, relativeTo: Date())
        let detail = sizeDetail
        let pin = isPinned ? " · Pinned" : ""
        if let app = sourceAppName {
            return "\(ago) · \(detail)\(pin) · \(app)"
        }
        return "\(ago) · \(detail)\(pin)"
    }

    public var sizeDetail: String {
        switch detectedKind {
        case .image:
            let bytes = imageData?.count ?? 0
            return bytes >= 1024 ? "\(bytes / 1024) KB image" : "\(bytes) B image"
        case .file:
            let count = fileURLs?.count ?? 0
            if count == 1, let name = fileURLs?.first.map({ URL(fileURLWithPath: $0).lastPathComponent }) {
                return name
            }
            return "\(count) files"
        case .color:
            if let colorHex { return colorHex }
            return "color"
        default:
            return "\(text.count) chars"
        }
    }

    public var symbolName: String {
        switch detectedKind {
        case .text: return "text.alignleft"
        case .link: return "link"
        case .email: return "envelope"
        case .code: return "chevron.left.forwardslash.chevron.right"
        case .image: return "photo"
        case .file: return "doc"
        case .color: return "paintpalette"
        }
    }

    public var plainTextForCopy: String {
        if detectedKind == .file, let fileURLs, !fileURLs.isEmpty {
            return fileURLs.joined(separator: "\n")
        }
        return text
    }

    public func searchHaystack() -> String {
        var parts = [
            text,
            sourceAppName ?? "",
            detectedKind.rawValue,
            colorHex ?? ""
        ]
        if let fileURLs {
            parts.append(contentsOf: fileURLs.map { URL(fileURLWithPath: $0).lastPathComponent })
        }
        return parts.joined(separator: " ").lowercased()
    }

    /// Collapsed preview for launcher result rows (newlines flattened).
    public var launcherPreviewText: String {
        let collapsed = displayText
            .replacingOccurrences(of: "\n", with: " ")
            .trimmingCharacters(in: .whitespacesAndNewlines)
        if collapsed.count <= 160 { return collapsed }
        return String(collapsed.prefix(160)) + "…"
    }
}
