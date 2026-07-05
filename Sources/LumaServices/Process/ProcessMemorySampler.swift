import AppKit
import Darwin
import Foundation
import LumaCore

/// Background sampler for resident memory; query paths read cached snapshots only.
public actor ProcessMemorySampler: ProcessMemoryClient {
    public static let shared = ProcessMemorySampler()

    private var cached: [RunningApplicationMemory] = []
    private var lastSampleAt = Date.distantPast
    private let sampleTTL: TimeInterval
    private var sampleTask: Task<Void, Never>?
    private var monitorTask: Task<Void, Never>?
    private var isStarted = false

    internal private(set) var sampleCallCount = 0
    internal private(set) var psInvocationCount = 0

    public init(sampleTTL: TimeInterval = 3.0) {
        self.sampleTTL = sampleTTL
    }

    public func start() async {
        guard !isStarted else { return }
        isStarted = true
        scheduleSample()
        monitorTask = Task { [weak self] in
            while !Task.isCancelled {
                try? await Task.sleep(for: .seconds(3))
                guard !Task.isCancelled, let self else { return }
                await self.sampleNow()
            }
        }
    }

    public func stop() {
        isStarted = false
        monitorTask?.cancel()
        monitorTask = nil
        sampleTask?.cancel()
        sampleTask = nil
    }

    public func topApplications(limit: Int) async -> [RunningApplicationMemory] {
        if cached.isEmpty || Date().timeIntervalSince(lastSampleAt) > sampleTTL {
            scheduleSample()
        }
        return Array(cached.prefix(limit))
    }

    internal func seedForTesting(_ samples: [RunningApplicationMemory], lastSampleAt: Date = Date()) {
        cached = samples
        self.lastSampleAt = lastSampleAt
    }

    private func scheduleSample() {
        guard sampleTask == nil else { return }
        sampleTask = Task { [weak self] in
            guard let self else { return }
            await self.sampleNow()
            await self.clearSampleTask()
        }
    }

    private func clearSampleTask() {
        sampleTask = nil
    }

    private func sampleNow() async {
        sampleCallCount += 1
        let processes = await MainActor.run { Self.runningProcesses() }
        var samples: [RunningApplicationMemory] = []
        samples.reserveCapacity(processes.count)
        var missing: Set<pid_t> = []
        var bytesByPID: [pid_t: UInt64] = [:]

        for process in processes {
            if let bytes = Self.residentBytes(pid: process.pid) {
                bytesByPID[process.pid] = bytes
            } else {
                missing.insert(process.pid)
            }
        }

        if !missing.isEmpty {
            psInvocationCount += 1
            let fallback = Self.residentBytesByPIDViaPS().filter { missing.contains($0.key) }
            for (pid, bytes) in fallback {
                bytesByPID[pid] = bytes
            }
        }

        for process in processes {
            guard let bytes = bytesByPID[process.pid] else { continue }
            samples.append(RunningApplicationMemory(
                bundleID: process.bundleID,
                name: process.name,
                residentBytes: bytes
            ))
        }

        cached = samples
            .sorted { $0.residentBytes > $1.residentBytes }
        lastSampleAt = Date()
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

/// Reads memory rankings from the shared background sampler.
public struct ProcessMemoryService: ProcessMemoryClient, Sendable {
    private let sampler: ProcessMemorySampler

    public init(sampler: ProcessMemorySampler = .shared) {
        self.sampler = sampler
    }

    public func topApplications(limit: Int) async -> [RunningApplicationMemory] {
        await sampler.topApplications(limit: limit)
    }
}
