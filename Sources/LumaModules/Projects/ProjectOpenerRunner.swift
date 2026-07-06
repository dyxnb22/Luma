import Foundation
import LumaCore

public enum ProjectOpenerRunner {
    public static func open(path: String, opener: ProjectOpener, workspace: any WorkspaceClient) async throws {
        let url = URL(fileURLWithPath: path, isDirectory: true)
        switch opener {
        case .cursor:
            try await runCLI(named: "cursor", arguments: [path], bundleID: "com.todesktop.230313mzl4w4u92", workspace: workspace)
        case .vscode:
            try await runCLI(named: "code", arguments: [path], bundleID: "com.microsoft.VSCode", workspace: workspace)
        case .finder:
            try await workspace.revealInFinder(url)
        case .terminal:
            let process = Process()
            process.executableURL = URL(fileURLWithPath: "/usr/bin/open")
            process.arguments = ["-a", "Terminal", path]
            try process.run()
        }
    }

    private static func runCLI(
        named: String,
        arguments: [String],
        bundleID: String,
        workspace: any WorkspaceClient
    ) async throws {
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

        await workspace.openApplication(bundleID: bundleID, arguments: arguments)
    }
}
