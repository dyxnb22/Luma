import AppKit
import Darwin
import Foundation
import LumaCore

/// Samples resident memory for running user applications via `proc_pidinfo`.
public struct ProcessMemoryService: ProcessMemoryClient, Sendable {
    public init() {}

    public func topApplications(limit: Int) async -> [RunningApplicationMemory] {
        let processes = await MainActor.run { Self.runningProcesses() }
        return Self.rank(processes: processes, limit: limit)
    }

    private struct ProcessInfo: Sendable {
        let bundleID: String
        let name: String
        let pid: pid_t
    }

    @MainActor
    private static func runningProcesses() -> [ProcessInfo] {
        NSWorkspace.shared.runningApplications.compactMap { app in
            guard app.activationPolicy == .regular,
                  let bundleID = app.bundleIdentifier else { return nil }
            let pid = app.processIdentifier
            guard pid > 0 else { return nil }
            return ProcessInfo(
                bundleID: bundleID,
                name: app.localizedName ?? bundleID,
                pid: pid
            )
        }
    }

    private static func rank(processes: [ProcessInfo], limit: Int) -> [RunningApplicationMemory] {
        var samples: [RunningApplicationMemory] = []
        samples.reserveCapacity(processes.count)
        for process in processes {
            guard let bytes = residentBytes(pid: process.pid) else { continue }
            samples.append(RunningApplicationMemory(
                bundleID: process.bundleID,
                name: process.name,
                residentBytes: bytes
            ))
        }
        return samples
            .sorted { $0.residentBytes > $1.residentBytes }
            .prefix(limit)
            .map { $0 }
    }

    private static func residentBytes(pid: pid_t) -> UInt64? {
        var info = proc_taskinfo()
        let size = proc_pidinfo(pid, PROC_PIDTASKINFO, 0, &info, Int32(MemoryLayout<proc_taskinfo>.size))
        guard size == MemoryLayout<proc_taskinfo>.size else { return nil }
        return UInt64(info.pti_resident_size)
    }
}
