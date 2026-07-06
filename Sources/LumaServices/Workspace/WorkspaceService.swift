import AppKit
import Foundation
import LumaCore

public struct WorkspaceService: WorkspaceClient, Sendable {
    public init() {}

    public func launchApplication(at url: URL) async throws {
        try await Self.launchAndActivateApplication(at: url)
    }

    public func openURL(_ url: URL) async throws {
        try ExternalURLPolicy.validateOpenURL(url, allowFileURLs: false)
        let opened = await MainActor.run {
            NSWorkspace.shared.open(url)
        }
        guard opened else {
            throw ModuleError.dataUnavailable
        }
    }

    public func openLocalFileURL(_ url: URL) async throws {
        guard url.isFileURL else {
            throw ExternalURLPolicyError.schemeNotAllowed(url.scheme ?? "(missing)")
        }
        let opened = await MainActor.run {
            NSWorkspace.shared.open(url)
        }
        guard opened else {
            throw ModuleError.dataUnavailable
        }
    }

    public func revealInFinder(_ url: URL) async throws {
        await MainActor.run {
            NSWorkspace.shared.activateFileViewerSelecting([url])
        }
    }

    public func terminateApplication(bundleID: String) async {
        _ = await MainActor.run {
            NSWorkspace.shared.runningApplications
                .first { $0.bundleIdentifier == bundleID }?
                .terminate()
        }
    }

    public func openApplication(bundleID: String, arguments: [String]) async {
        await MainActor.run {
            guard let appURL = NSWorkspace.shared.urlForApplication(withBundleIdentifier: bundleID) else {
                if let path = arguments.first {
                    NSWorkspace.shared.open(URL(fileURLWithPath: path, isDirectory: true))
                }
                return
            }
            let config = NSWorkspace.OpenConfiguration()
            config.arguments = arguments
            NSWorkspace.shared.openApplication(at: appURL, configuration: config) { _, _ in }
        }
    }

    @MainActor
    private static func launchAndActivateApplication(at url: URL) async throws {
        if let running = runningApplication(for: url) {
            activate(running)
            return
        }

        let configuration = NSWorkspace.OpenConfiguration()
        configuration.activates = true
        configuration.addsToRecentItems = true

        let launched: NSRunningApplication? = try await withCheckedThrowingContinuation { continuation in
            NSWorkspace.shared.openApplication(at: url, configuration: configuration) { app, error in
                if let error {
                    continuation.resume(throwing: error)
                    return
                }
                continuation.resume(returning: app)
            }
        }

        if let launched {
            activate(launched)
        } else if let running = runningApplication(for: url) {
            activate(running)
        }
    }

    @MainActor
    private static func runningApplication(for url: URL) -> NSRunningApplication? {
        let targetPath = url.resolvingSymlinksInPath().standardizedFileURL.path
        return NSWorkspace.shared.runningApplications.first { app in
            app.bundleURL?.resolvingSymlinksInPath().standardizedFileURL.path == targetPath
        }
    }

    @MainActor
    private static func activate(_ app: NSRunningApplication) {
        app.unhide()
        app.activate(from: NSRunningApplication.current, options: [.activateAllWindows])
    }
}
