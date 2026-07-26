import Darwin
import Foundation
import LumaWorkbenchCore

/// Samples combined host/child RSS and trims terminal scrollback when macOS reports pressure.
/// Metrics stay local and are written only to the unified log; there is no telemetry/export path.
final class MemoryPressureController {
    private weak var session: TerminalSessionController?
    private var tracker = ResidentMemoryTracker()
    private var sampleTimer: DispatchSourceTimer?
    private var pressureSource: DispatchSourceMemoryPressure?

    init(session: TerminalSessionController) {
        self.session = session
    }

    func start() {
        sample()

        let timer = DispatchSource.makeTimerSource(queue: .main)
        timer.schedule(
            deadline: .now() + MemoryPressurePolicy.samplingInterval,
            repeating: MemoryPressurePolicy.samplingInterval
        )
        timer.setEventHandler { [weak self] in self?.sample() }
        timer.resume()
        sampleTimer = timer

        let pressure = DispatchSource.makeMemoryPressureSource(
            eventMask: [.warning, .critical],
            queue: .main
        )
        pressure.setEventHandler { [weak self, weak pressure] in
            guard let pressure else { return }
            let level: MemoryPressureLevel = pressure.data.contains(.critical) ? .critical : .warning
            self?.handle(level)
        }
        pressure.resume()
        pressureSource = pressure
    }

    func stopAndLogPeak() {
        sample()
        sampleTimer?.cancel()
        pressureSource?.cancel()
        sampleTimer = nil
        pressureSource = nil
        NSLog(
            "Luma memory: peak combined RSS %@",
            ByteCountFormatter.string(fromByteCount: Int64(clamping: tracker.peakTotalBytes), countStyle: .memory)
        )
    }

    func sampleNow() {
        sample()
    }

    private func handle(_ level: MemoryPressureLevel) {
        session?.applyMemoryPressure(level)
        sample()
        let current = tracker.latest?.totalBytes ?? 0
        NSLog(
            "Luma memory pressure %@: combined RSS %@, peak %@; terminal scrollback reduced to %d lines",
            String(describing: level),
            ByteCountFormatter.string(fromByteCount: Int64(clamping: current), countStyle: .memory),
            ByteCountFormatter.string(fromByteCount: Int64(clamping: tracker.peakTotalBytes), countStyle: .memory),
            MemoryPressurePolicy.scrollbackLines(for: level)
        )
    }

    private func sample() {
        let host = residentBytes(processIdentifier: ProcessInfo.processInfo.processIdentifier) ?? 0
        let child = session?.processIdentifier.flatMap(residentBytes(processIdentifier:)) ?? 0
        tracker.record(.init(hostBytes: host, childBytes: child))
    }

    private func residentBytes(processIdentifier: pid_t) -> UInt64? {
        guard processIdentifier > 0 else { return nil }
        var info = proc_taskinfo()
        let expected = Int32(MemoryLayout<proc_taskinfo>.size)
        let read = withUnsafeMutablePointer(to: &info) { pointer in
            proc_pidinfo(processIdentifier, PROC_PIDTASKINFO, 0, pointer, expected)
        }
        guard read == expected else { return nil }
        return info.pti_resident_size
    }
}
