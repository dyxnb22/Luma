import Foundation
import Testing

@Test func documentationDoesNotReuseDeprecatedPhrases() throws {
    let repoRoot = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()

    let bannedPhrases = [
        "360 pt",
        "360pt",
        "all discoverable prefixes",
        "module command catalog",
        "session save uses empty field",
        "533 tests",
        "552 tests",
        "FeatureCatalog.swift",
        "sectioned home rows",
        "single-column list with"
    ]

    let scanRoots = [
        repoRoot.appendingPathComponent("docs"),
        repoRoot.appendingPathComponent(".cursor/rules")
    ]

    var violations: [String] = []
    let manager = FileManager.default

    for root in scanRoots {
        guard let enumerator = manager.enumerator(
            at: root,
            includingPropertiesForKeys: [.isRegularFileKey],
            options: [.skipsHiddenFiles]
        ) else {
            continue
        }

        for case let fileURL as URL in enumerator {
            let ext = fileURL.pathExtension.lowercased()
            guard ext == "md" || ext == "mdc" else { continue }
            let relativePath = fileURL.path.replacingOccurrences(of: repoRoot.path + "/", with: "")
            if DocumentationDriftAllowlist.matches(relativePath) { continue }

            let text = try String(contentsOf: fileURL, encoding: .utf8)
            for phrase in bannedPhrases where text.localizedCaseInsensitiveContains(phrase) {
                violations.append("\(relativePath): \"\(phrase)\"")
            }
        }
    }

    if !violations.isEmpty {
        Issue.record("Deprecated documentation phrases found:\n\(violations.joined(separator: "\n"))")
    }
    #expect(violations.isEmpty)
}

private enum DocumentationDriftAllowlist {
    /// Intentional historical references may be listed here with a short reason in a nearby comment.
    static func matches(_ relativePath: String) -> Bool {
        _ = relativePath
        return false
    }
}
