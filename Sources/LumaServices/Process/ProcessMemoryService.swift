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
        let psFallback = residentBytesByPIDViaPS()
        for process in processes {
            let bytes = residentBytes(pid: process.pid) ?? psFallback[process.pid]
            guard let bytes else { continue }
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

    /// Fallback when `proc_pidinfo` is unavailable (e.g. hardened runtime without task access).
    private static func residentBytesByPIDViaPS() -> [pid_t: UInt64] {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/bin/ps")
        process.arguments = ["-axo", "pid=,rss="]
        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = Pipe()
        guard (try? process.run()) != nil else { return [:] }
        process.waitUntilExit()
        guard process.terminationStatus == 0 else { return [:] }
        guard let output = String(data: pipe.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) else {
            return [:]
        }
        var map: [pid_t: UInt64] = [:]
        for line in output.split(whereSeparator: \.isNewline) {
            let parts = line.split(whereSeparator: \.isWhitespace).map(String.init)
            guard parts.count >= 2,
                  let pid = pid_t(parts[0]),
                  let rssKB = UInt64(parts[1]) else { continue }
            map[pid] = rssKB * 1024
        }
        return map
    }

    private static func residentBytes(pid: pid_t) -> UInt64? {
        var info = proc_taskinfo()
        let size = proc_pidinfo(pid, PROC_PIDTASKINFO, 0, &info, Int32(MemoryLayout<proc_taskinfo>.size))
        guard size == MemoryLayout<proc_taskinfo>.size else { return nil }
        return UInt64(info.pti_resident_size)
    }
}
