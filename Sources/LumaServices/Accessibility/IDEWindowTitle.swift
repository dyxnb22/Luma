import Foundation

/// Extracts a human-friendly workspace / project label from IDE window titles for the Open Apps sidebar.
public enum IDEWindowTitle {
    private static let electronBundleIDPrefixes = [
        "com.todesktop.",          // Cursor
        "com.microsoft.VSCode",    // VS Code
        "com.openai.codex",        // Codex
        "com.github.atom",         // Atom
        "com.sublimetext.",        // Sublime Text
    ]

    private static let jetBrainsBundleIDPrefixes = [
        "com.jetbrains.",
    ]

    private static let xcodeBundleIDs: Set<String> = [
        "com.apple.dt.Xcode",
    ]

    private static let knownAppNames: Set<String> = [
        "cursor",
        "visual studio code",
        "vscode",
        "vs code",
        "codex",
        "intellij idea",
        "intellij idea ce",
        "intellij",
        "xcode",
        "webstorm",
        "pycharm",
        "goland",
        "clion",
        "rider",
        "phpstorm",
        "rubymine",
        "datagrip",
        "appcode",
        "android studio",
        "fleet",
    ]

    private static let sourceExtensions: Set<String> = [
        "c", "cc", "cpp", "cs", "css", "go", "h", "hpp", "html", "java", "js", "jsx",
        "json", "kt", "kts", "m", "md", "mm", "php", "py", "rb", "rs", "scss", "sh",
        "sql", "swift", "toml", "ts", "tsx", "txt", "vue", "xml", "yaml", "yml", "zsh",
    ]

    /// Returns a sidebar label for an IDE window, or the raw title when parsing does not apply.
    public static func sidebarLabel(rawTitle: String, bundleID: String, appName: String) -> String {
        let trimmed = rawTitle.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return rawTitle }
        guard isIDE(bundleID: bundleID) else { return rawTitle }

        let appNames = appNameTokens(appName: appName, bundleID: bundleID)
        let segments = splitSegments(trimmed)
        guard !segments.isEmpty else { return rawTitle }
        guard let family = family(for: bundleID) else { return rawTitle }

        switch family {
        case .jetBrains:
            if let project = segments.first(where: { !isAppName($0, appNames: appNames) && !looksLikeFilename($0) }) {
                return project
            }
        case .electron, .xcode:
            let candidates = segments.filter { !isAppName($0, appNames: appNames) && !looksLikeFilename($0) }
            if let project = candidates.last {
                return project
            }
            if segments.count >= 2,
               looksLikeFilename(segments[0]),
               !isAppName(segments[1], appNames: appNames) {
                return segments[1]
            }
        }

        if segments.count == 1, !isAppName(segments[0], appNames: appNames) {
            return segments[0]
        }

        return rawTitle
    }

    public static func isIDE(bundleID: String) -> Bool {
        family(for: bundleID) != nil
    }

    private enum Family {
        case electron
        case jetBrains
        case xcode
    }

    private static func family(for bundleID: String) -> Family? {
        if xcodeBundleIDs.contains(bundleID) { return .xcode }
        if jetBrainsBundleIDPrefixes.contains(where: { bundleID.hasPrefix($0) }) { return .jetBrains }
        if electronBundleIDPrefixes.contains(where: { bundleID.hasPrefix($0) || bundleID == $0 }) { return .electron }
        return nil
    }

    private static func appNameTokens(appName: String, bundleID: String) -> Set<String> {
        var names = knownAppNames
        names.insert(appName.lowercased())
        if bundleID.hasPrefix("com.todesktop.") {
            names.insert("cursor")
        }
        return names
    }

    private static func splitSegments(_ title: String) -> [String] {
        title
            .replacingOccurrences(of: " – ", with: " — ")
            .replacingOccurrences(of: " - ", with: " — ")
            .split(separator: "—", omittingEmptySubsequences: false)
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
    }

    private static func isAppName(_ segment: String, appNames: Set<String>) -> Bool {
        let normalized = segment.lowercased()
        return appNames.contains(normalized)
    }

    private static func looksLikeFilename(_ segment: String) -> Bool {
        let name = segment.split(separator: "/").last.map(String.init) ?? segment
        guard let dot = name.lastIndex(of: "."), dot > name.startIndex else { return false }
        let ext = name[name.index(after: dot)...].lowercased()
        return sourceExtensions.contains(ext)
    }
}
