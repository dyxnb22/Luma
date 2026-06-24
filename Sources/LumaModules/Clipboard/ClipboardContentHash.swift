import CryptoKit
import Foundation

enum ClipboardContentHash {
    static func compute(
        text: String,
        imageData: Data?,
        fileURLs: [String]?,
        colorHex: String?
    ) -> String {
        var parts: [String] = []
        if let imageData, !imageData.isEmpty {
            parts.append("img:\(SHA256.hash(data: imageData).hexString)")
        }
        if let fileURLs, !fileURLs.isEmpty {
            let normalized = fileURLs.map { URL(fileURLWithPath: $0).standardizedFileURL.path }.sorted()
            parts.append("file:\(normalized.joined(separator: "\u{1f}"))")
        }
        if let colorHex, !colorHex.isEmpty {
            parts.append("color:\(colorHex.lowercased())")
        }
        if parts.isEmpty {
            parts.append("text:\(text)")
        }
        return SHA256.hash(data: Data(parts.joined(separator: "|").utf8)).hexString
    }

    static func backfill(for entry: ClipboardEntry) -> String {
        compute(
            text: entry.text,
            imageData: entry.imageData,
            fileURLs: entry.fileURLs,
            colorHex: entry.colorHex
        )
    }
}

private extension SHA256.Digest {
    var hexString: String {
        map { String(format: "%02x", $0) }.joined()
    }
}
