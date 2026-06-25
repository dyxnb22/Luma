import Foundation

public struct ClipboardSnapshot: Sendable, Equatable {
    public let changeCount: Int
    public let types: [String]
    public let text: String?
    public let imageData: Data?
    public let imageType: String?
    public let fileURLs: [URL]
    public let sourceAppName: String?
    public let sourceBundleID: String?
    public let sourceIsLuma: Bool

    public init(
        changeCount: Int,
        types: [String],
        text: String?,
        imageData: Data?,
        imageType: String?,
        fileURLs: [URL],
        sourceAppName: String?,
        sourceBundleID: String?,
        sourceIsLuma: Bool = false
    ) {
        self.changeCount = changeCount
        self.types = types
        self.text = text
        self.imageData = imageData
        self.imageType = imageType
        self.fileURLs = fileURLs
        self.sourceAppName = sourceAppName
        self.sourceBundleID = sourceBundleID
        self.sourceIsLuma = sourceIsLuma
    }
}

#if canImport(AppKit)
import AppKit

public enum ClipboardSnapshotReader {
    public static func read(lumaBundleID: String?) -> ClipboardSnapshot {
        let pasteboard = NSPasteboard.general
        let changeCount = pasteboard.changeCount
        let types = pasteboard.types?.map(\.rawValue) ?? []
        let frontmost = NSWorkspace.shared.frontmostApplication
        let bundleID = frontmost?.bundleIdentifier
        let appName = frontmost?.localizedName
        let skipSource = bundleID == lumaBundleID

        var fileURLs: [URL] = []
        if ClipboardEntryKind.isFileTypes(types) {
            if let urls = pasteboard.readObjects(forClasses: [NSURL.self], options: nil) as? [URL] {
                fileURLs = urls
            }
        }

        var imageData: Data?
        var imageType: String?
        if ClipboardEntryKind.isImageTypes(types) {
            for type in ["public.png", "public.tiff", "public.jpeg", "public.image"] {
                if let data = pasteboard.data(forType: NSPasteboard.PasteboardType(type)) {
                    imageData = data
                    imageType = type
                    break
                }
            }
            if imageData == nil, let data = pasteboard.data(forType: .tiff) {
                imageData = data
                imageType = NSPasteboard.PasteboardType.tiff.rawValue
            }
        }

        let text = Self.readText(from: pasteboard)

        return ClipboardSnapshot(
            changeCount: changeCount,
            types: types,
            text: text,
            imageData: imageData,
            imageType: imageType,
            fileURLs: fileURLs,
            sourceAppName: skipSource ? nil : appName,
            sourceBundleID: skipSource ? nil : bundleID,
            sourceIsLuma: skipSource
        )
    }

    private static let textPasteboardTypes: [NSPasteboard.PasteboardType] = [
        .string,
        NSPasteboard.PasteboardType("NSStringPboardType"),
        NSPasteboard.PasteboardType("public.plain-text")
    ]

    private static func readText(from pasteboard: NSPasteboard) -> String? {
        for type in textPasteboardTypes {
            if let text = pasteboard.string(forType: type), !text.isEmpty {
                return text
            }
        }
        if let strings = pasteboard.readObjects(forClasses: [NSString.self], options: nil) as? [String],
           let first = strings.first(where: { !$0.isEmpty }) {
            return first
        }
        if let rtf = pasteboard.data(forType: .rtf),
           let attributed = NSAttributedString(rtf: rtf, documentAttributes: nil),
           !attributed.string.isEmpty {
            return attributed.string
        }
        if let htmlData = pasteboard.data(forType: .html),
           let attributed = try? NSAttributedString(
               data: htmlData,
               options: [
                   .documentType: NSAttributedString.DocumentType.html,
                   .characterEncoding: String.Encoding.utf8.rawValue
               ],
               documentAttributes: nil
           ),
           !attributed.string.isEmpty {
            return attributed.string
        }
        return nil
    }
}
#endif
