import AppKit
import Foundation

public enum ProjectOpenerRunner {
    public static func open(path: String, opener: ProjectOpener) async throws {
        let url = URL(fileURLWithPath: path, isDirectory: true)
        switch opener {
        case .cursor:
            try await runCLI(named: "cursor", arguments: [path], bundleID: "com.todesktop.230313mzl4w4u92")
        case .vscode:
            try await runCLI(named: "code", arguments: [path], bundleID: "com.microsoft.VSCode")
        case .finder:
            await MainActor.run {
                NSWorkspace.shared.activateFileViewerSelecting([url])
            }
        case .terminal:
            let process = Process()
            process.executableURL = URL(fileURLWithPath: "/usr/bin/open")
            process.arguments = ["-a", "Terminal", path]
            try process.run()
        }
    }

    private static func runCLI(named: String, arguments: [String], bundleID: String) async throws {
        let candidates = [
            "/opt/homebrew/bin/\(named)",
            "/usr/local/bin/\(named)"
        ]
        for candidate in candidates where FileManager.default.isExecutableFile(atPath: candidate) {
            let process = Process()
            process.executableURL = URL(fileURLWithPath: candidate)
            process.arguments = arguments
            try process.run()
            return
        }

        await MainActor.run {
            if let appURL = NSWorkspace.shared.urlForApplication(withBundleIdentifier: bundleID) {
                let config = NSWorkspace.OpenConfiguration()
                config.arguments = arguments
                NSWorkspace.shared.openApplication(at: appURL, configuration: config) { _, _ in }
            } else if let path = arguments.first {
                NSWorkspace.shared.open(urlForPath(path))
            }
        }
    }

    private static func urlForPath(_ path: String) -> URL {
        URL(fileURLWithPath: path, isDirectory: true)
    }
}
