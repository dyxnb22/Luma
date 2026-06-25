import Foundation

/// Title-based window resolution for Open Apps and window focus actions.
enum WindowFocusMatcher {
    typealias TitledWindow = (index: Int, title: String)

    static func matchingIndex(
        in titledWindows: [TitledWindow],
        queryTitle: String,
        bundleID: String,
        appName: String
    ) -> Int? {
        let normalized = queryTitle.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !normalized.isEmpty else {
            return titledWindows.count == 1 ? titledWindows[0].index : nil
        }

        for (arrayIndex, windowTitle) in titledWindows.map(\.title).enumerated() where windowTitle == normalized {
            return titledWindows[arrayIndex].index
        }

        if IDEWindowTitle.isIDE(bundleID: bundleID) {
            let targetLabels = labelCandidates(for: normalized, bundleID: bundleID, appName: appName)
            for (arrayIndex, windowTitle) in titledWindows.map(\.title).enumerated() {
                let label = IDEWindowTitle.sidebarLabel(rawTitle: windowTitle, bundleID: bundleID, appName: appName)
                if targetLabels.contains(label) {
                    return titledWindows[arrayIndex].index
                }
            }
        }

        for (arrayIndex, windowTitle) in titledWindows.map(\.title).enumerated() {
            if titlesMatch(windowTitle, normalized, bundleID: bundleID, appName: appName) {
                return titledWindows[arrayIndex].index
            }
        }

        return titledWindows.count == 1 ? titledWindows[0].index : nil
    }

    static func labelCandidates(for title: String, bundleID: String, appName: String) -> Set<String> {
        var labels: Set<String> = [title]
        labels.insert(IDEWindowTitle.sidebarLabel(rawTitle: title, bundleID: bundleID, appName: appName))
        return labels
    }

    static func titlesMatch(
        _ lhs: String,
        _ rhs: String,
        bundleID: String,
        appName: String
    ) -> Bool {
        if lhs.localizedCaseInsensitiveCompare(rhs) == .orderedSame { return true }
        let left = lhs.lowercased()
        let right = rhs.lowercased()
        if left.contains(right) || right.contains(left) { return true }

        if IDEWindowTitle.isIDE(bundleID: bundleID) {
            let leftLabel = IDEWindowTitle.sidebarLabel(rawTitle: lhs, bundleID: bundleID, appName: appName)
            let rightLabel = IDEWindowTitle.sidebarLabel(rawTitle: rhs, bundleID: bundleID, appName: appName)
            if !leftLabel.isEmpty, leftLabel == rightLabel { return true }
        }

        let leftSegments = titleSegments(lhs)
        let rightSegments = titleSegments(rhs)
        guard let leftProject = projectSegment(in: leftSegments, bundleID: bundleID, appName: appName),
              let rightProject = projectSegment(in: rightSegments, bundleID: bundleID, appName: appName) else {
            return false
        }
        return leftProject == rightProject
    }

    private static func projectSegment(in segments: [String], bundleID: String, appName: String) -> String? {
        let appNames = appNameTokens(appName: appName, bundleID: bundleID)
        let candidates = segments.filter { !appNames.contains($0.lowercased()) }
        return candidates.last ?? segments.last
    }

    private static func appNameTokens(appName: String, bundleID: String) -> Set<String> {
        var names: Set<String> = [appName.lowercased(), "cursor", "visual studio code", "vscode", "vs code"]
        if bundleID.hasPrefix("com.todesktop.") {
            names.insert("cursor")
        }
        return names
    }

    private static func titleSegments(_ title: String) -> [String] {
        title
            .replacingOccurrences(of: " – ", with: " — ")
            .replacingOccurrences(of: " - ", with: " — ")
            .split(separator: "—", omittingEmptySubsequences: false)
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() }
            .filter { !$0.isEmpty }
    }
}
