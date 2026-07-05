import AppKit
import Darwin
import Foundation
import LumaCore

public struct RunningProcessRecord: Sendable, Hashable {
    public let pid: pid_t
    public let bundleID: String
    public let name: String
    public let launchDate: Date?
    public let residentBytes: UInt64?

    public init(pid: pid_t, bundleID: String, name: String, launchDate: Date?, residentBytes: UInt64?) {
        self.pid = pid
        self.bundleID = bundleID
        self.name = name
        self.launchDate = launchDate
        self.residentBytes = residentBytes
    }

    public var residentMB: Double? {
        residentBytes.map { Double($0) / 1_048_576.0 }
    }
}

struct ProcessMetadata: Sendable, Hashable {
    let pid: pid_t
    let bundleID: String
    let name: String
    let launchDate: Date?
}

public struct RunningProcessService: Sendable {
    public init() {}

    public func runningGUIApplications() async -> [RunningProcessRecord] {
        let metadata = await MainActor.run { Self.collectGUIMetadata() }
        return await enrichWithMemory(metadata)
    }

    @MainActor
    private static func collectGUIMetadata() -> [ProcessMetadata] {
        let selfPID = ProcessInfo.processInfo.processIdentifier
        return NSWorkspace.shared.runningApplications.compactMap { app in
            guard app.activationPolicy == .regular,
                  app.processIdentifier != selfPID,
                  let bundleID = app.bundleIdentifier else { return nil }
            return ProcessMetadata(
                pid: app.processIdentifier,
                bundleID: bundleID,
                name: app.localizedName ?? bundleID,
                launchDate: app.launchDate
            )
        }
    }

    private func enrichWithMemory(_ metadata: [ProcessMetadata]) async -> [RunningProcessRecord] {
        await Task.detached(priority: .utility) {
            metadata.map { item in
                RunningProcessRecord(
                    pid: item.pid,
                    bundleID: item.bundleID,
                    name: item.name,
                    launchDate: item.launchDate,
                    residentBytes: Self.residentBytes(pid: item.pid)
                )
            }
        }.value
    }

    public func quit(pid: pid_t) async -> Bool {
        await MainActor.run {
            NSRunningApplication(processIdentifier: pid)?.terminate() ?? false
        }
    }

    public func forceKill(pid: pid_t) async -> Bool {
        await MainActor.run {
            NSRunningApplication(processIdentifier: pid)?.forceTerminate() ?? false
        }
    }

    public func relaunch(bundleID: String, previousPID: pid_t, workspace: any WorkspaceClient) async {
        _ = await quit(pid: previousPID)
        let deadline = Date().addingTimeInterval(3)
        while Date() < deadline {
            let stillRunning = await MainActor.run {
                NSRunningApplication(processIdentifier: previousPID) != nil
            }
            if !stillRunning { break }
            try? await Task.sleep(for: .milliseconds(100))
        }
        await workspace.openApplication(bundleID: bundleID, arguments: [])
    }

    private static func residentBytes(pid: pid_t) -> UInt64? {
        var info = proc_taskinfo()
        let size = proc_pidinfo(pid, PROC_PIDTASKINFO, 0, &info, Int32(MemoryLayout<proc_taskinfo>.size))
        guard size == MemoryLayout<proc_taskinfo>.size else { return nil }
        return UInt64(info.pti_resident_size)
    }
}
