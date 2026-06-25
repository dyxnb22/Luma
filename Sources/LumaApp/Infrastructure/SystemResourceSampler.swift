import Darwin
import Foundation
import LumaCore

struct SystemResourceSnapshot: Sendable, Equatable {
    let cpuPercent: Double?
    let usedMemoryBytes: UInt64
    let totalMemoryBytes: UInt64
    let timestamp: Date

    var memoryUsageRatio: Double {
        guard totalMemoryBytes > 0 else { return 0 }
        return Double(usedMemoryBytes) / Double(totalMemoryBytes)
    }
}

enum PerformanceStripEmphasis: Sendable, Equatable {
    case normal
    case elevated
    case warning
}

struct PerformanceStripSummarySnapshot: Sendable, Equatable {
    let todayCount: Int?
    let reviewCount: Int?

    static let empty = PerformanceStripSummarySnapshot(todayCount: nil, reviewCount: nil)
}

struct PerformanceStripPresentation: Sendable, Equatable {
    let cpuText: String
    let memoryText: String
    let todayText: String
    let reviewText: String
    let cpuEmphasis: PerformanceStripEmphasis
    let memoryEmphasis: PerformanceStripEmphasis
    let todayEmphasis: PerformanceStripEmphasis
    let reviewEmphasis: PerformanceStripEmphasis
}

final class SystemResourceProbe: @unchecked Sendable {
    private let lock = NSLock()
    private var previousCPUTicks: (user: UInt32, system: UInt32, idle: UInt32, nice: UInt32)?

    func sample() -> SystemResourceSnapshot {
        let cpu = readCPUPercent()
        let memory = readMemoryBytes()
        return SystemResourceSnapshot(
            cpuPercent: cpu >= 0 ? cpu : nil,
            usedMemoryBytes: memory.used,
            totalMemoryBytes: memory.total,
            timestamp: Date()
        )
    }

    private func readCPUPercent() -> Double {
        var loadInfo = host_cpu_load_info()
        var count = mach_msg_type_number_t(
            MemoryLayout<host_cpu_load_info_data_t>.size / MemoryLayout<integer_t>.size
        )
        let result = withUnsafeMutablePointer(to: &loadInfo) {
            $0.withMemoryRebound(to: integer_t.self, capacity: Int(count)) {
                host_statistics(mach_host_self(), HOST_CPU_LOAD_INFO, $0, &count)
            }
        }
        guard result == KERN_SUCCESS else { return 0 }

        let ticks = loadInfo.cpu_ticks
        let current = (user: ticks.0, system: ticks.1, idle: ticks.2, nice: ticks.3)

        lock.lock()
        defer { lock.unlock() }

        guard let previous = previousCPUTicks else {
            previousCPUTicks = current
            return -1
        }

        let user = Double(current.user &- previous.user)
        let system = Double(current.system &- previous.system)
        let idle = Double(current.idle &- previous.idle)
        let nice = Double(current.nice &- previous.nice)
        previousCPUTicks = current

        let total = user + system + idle + nice
        guard total > 0 else { return 0 }
        return ((user + system + nice) / total) * 100
    }

    private func readMemoryBytes() -> (used: UInt64, total: UInt64) {
        let total = ProcessInfo.processInfo.physicalMemory
        var stats = vm_statistics64()
        var count = mach_msg_type_number_t(
            MemoryLayout<vm_statistics64>.size / MemoryLayout<integer_t>.size
        )
        let result = withUnsafeMutablePointer(to: &stats) {
            $0.withMemoryRebound(to: integer_t.self, capacity: Int(count)) {
                host_statistics64(mach_host_self(), HOST_VM_INFO64, $0, &count)
            }
        }
        guard result == KERN_SUCCESS else { return (0, total) }

        let pageSize = UInt64(sysconf(_SC_PAGESIZE))
        let usedPages = UInt64(stats.active_count)
            + UInt64(stats.wire_count)
            + UInt64(stats.compressor_page_count)
        return (usedPages * pageSize, total)
    }
}

@MainActor
enum PerformanceStripFormatter {
    static func format(
        _ snapshot: SystemResourceSnapshot,
        summary: PerformanceStripSummarySnapshot,
        smoothedCPU: inout Double?
    ) -> PerformanceStripPresentation {
        let cpuText: String
        let cpu: Double?
        if let rawCPU = snapshot.cpuPercent {
            let nextCPU: Double
            if let previous = smoothedCPU {
                nextCPU = previous * 0.65 + rawCPU * 0.35
            } else {
                nextCPU = rawCPU
            }
            smoothedCPU = nextCPU
            cpu = nextCPU
            cpuText = "\(Int(nextCPU.rounded()))%"
        } else {
            cpu = nil
            cpuText = "—"
        }
        let usedGB = Double(snapshot.usedMemoryBytes) / 1_073_741_824
        let totalGB = Double(snapshot.totalMemoryBytes) / 1_073_741_824
        let memoryText = String(format: "%.1f / %.0f GB", usedGB, totalGB)
        let todayText = summary.todayCount.map(String.init) ?? "—"
        let reviewText = summary.reviewCount.map(String.init) ?? "—"

        return PerformanceStripPresentation(
            cpuText: cpuText,
            memoryText: memoryText,
            todayText: todayText,
            reviewText: reviewText,
            cpuEmphasis: {
                guard let cpu else { return .normal }
                return cpu >= LauncherChromeTokens.performanceWarningCPU ? .warning : .normal
            }(),
            memoryEmphasis: snapshot.memoryUsageRatio >= LauncherChromeTokens.performanceWarningMemoryRatio ? .warning : .normal,
            todayEmphasis: (summary.todayCount ?? 0) > 0 ? .elevated : .normal,
            reviewEmphasis: (summary.reviewCount ?? 0) > 0 ? .elevated : .normal
        )
    }
}

@MainActor
final class SystemResourceSampler {
    var onUpdate: ((PerformanceStripPresentation) -> Void)?
    var summaryProvider: (@Sendable () async -> PerformanceStripSummarySnapshot)?

    private var timer: Timer?
    private var smoothedCPU: Double?
    private let probe = SystemResourceProbe()
    private var cachedSummary = PerformanceStripSummarySnapshot.empty
    private var lastSummaryRefreshAt: Date?
    private static let summaryRefreshInterval: TimeInterval = 30

    func start() {
        guard timer == nil else { return }
        smoothedCPU = nil
        cachedSummary = .empty
        lastSummaryRefreshAt = nil
        tick(forceSummaryRefresh: true)
        timer = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { [weak self] _ in
            Task { @MainActor in self?.tick() }
        }
    }

    func stop() {
        timer?.invalidate()
        timer = nil
        smoothedCPU = nil
        cachedSummary = .empty
        lastSummaryRefreshAt = nil
    }

    private func tick(forceSummaryRefresh: Bool = false) {
        let probe = probe
        let summaryProvider = summaryProvider
        let cachedSummary = cachedSummary
        let shouldRefreshSummary = forceSummaryRefresh
            || lastSummaryRefreshAt == nil
            || Date().timeIntervalSince(lastSummaryRefreshAt ?? .distantPast) >= Self.summaryRefreshInterval
        Task.detached(priority: .utility) {
            let snapshot = probe.sample()
            let summary: PerformanceStripSummarySnapshot
            if shouldRefreshSummary, let summaryProvider {
                summary = await summaryProvider()
            } else {
                summary = cachedSummary
            }
            await MainActor.run { [weak self] in
                guard let self else { return }
                if shouldRefreshSummary {
                    self.cachedSummary = summary
                    self.lastSummaryRefreshAt = Date()
                }
                let presentation = PerformanceStripFormatter.format(
                    snapshot,
                    summary: self.cachedSummary,
                    smoothedCPU: &self.smoothedCPU
                )
                self.onUpdate?(presentation)
            }
        }
    }
}
