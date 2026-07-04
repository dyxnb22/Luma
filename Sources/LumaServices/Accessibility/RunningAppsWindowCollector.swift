import Foundation

/// Enriches running-app metadata with per-window AX snapshots off the MainActor hot path.
public enum RunningAppsWindowCollector {
    public static func windowsByPID(
        for apps: [RunningAppMetadata],
        using enumerator: any AXWindowEnumerating
    ) -> [Int32: [OpenWindowSnapshot]] {
        guard enumerator.isAccessibilityGranted, !apps.isEmpty else { return [:] }
        let cgWindowsByPID = enumerator.copyOnScreenWindowsByPID()
        var result: [Int32: [OpenWindowSnapshot]] = [:]
        for app in apps {
            let cgWindows = cgWindowsByPID[app.pid] ?? []
            let windows = enumerator.enumerateWindows(for: app.pid, appName: app.name, cgWindows: cgWindows)
            if !windows.isEmpty {
                result[app.pid] = windows
            }
        }
        return result
    }
}
