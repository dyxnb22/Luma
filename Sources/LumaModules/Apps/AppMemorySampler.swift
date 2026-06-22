import AppKit
import Darwin
import Foundation

public struct AppMemorySample: Sendable, Hashable {
    public let bundleID: String
    public let name: String
    public let residentBytes: UInt64

    public var residentMB: Double {
        Double(residentBytes) / 1_048_576.0
    }
}

/// Samples resident memory for running user applications via `proc_pidinfo`.
public enum AppMemorySampler {
    public static func topApplications(limit: Int = 8) -> [AppMemorySample] {
        let apps = NSWorkspace.shared.runningApplications.filter {
            $0.activationPolicy == .regular && $0.bundleIdentifier != nil
        }
        var samples: [AppMemorySample] = []
        for app in apps {
            guard let bundleID = app.bundleIdentifier else { continue }
            let pid = app.processIdentifier
            guard pid > 0, let bytes = residentBytes(pid: pid) else { continue }
            samples.append(AppMemorySample(
                bundleID: bundleID,
                name: app.localizedName ?? bundleID,
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
