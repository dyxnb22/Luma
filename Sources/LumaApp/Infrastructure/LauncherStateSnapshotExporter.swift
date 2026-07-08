import Foundation
import LumaCore
import LumaInfrastructure

/// Writes launcher state snapshots and invariant violations for QA (`LUMA_QA=1`).
@MainActor
enum LauncherStateSnapshotExporter {
    private static let lock = NSLock()
    nonisolated(unsafe) private static var debounceWorkItem: DispatchWorkItem?
    private static let debounceInterval: TimeInterval = 0.2

    static var isQAEnabled: Bool {
        ProcessInfo.processInfo.environment["LUMA_QA"] == "1"
            || ProcessInfo.processInfo.environment["LUMA_QA_EXPORT_STATE"] == "1"
    }

    static func exportNow(_ snapshot: LauncherStateSnapshot) {
        guard isQAEnabled else { return }
        writeSnapshot(snapshot)
        recordViolations(for: snapshot)
    }

    static func scheduleExport(_ snapshotProvider: @escaping @MainActor () -> LauncherStateSnapshot) {
        guard isQAEnabled else { return }
        lock.lock()
        debounceWorkItem?.cancel()
        let work = DispatchWorkItem {
            Task { @MainActor in
                exportNow(snapshotProvider())
            }
        }
        debounceWorkItem = work
        lock.unlock()
        DispatchQueue.main.asyncAfter(deadline: .now() + debounceInterval, execute: work)
    }

    private static func writeSnapshot(_ snapshot: LauncherStateSnapshot) {
        guard let url = logsDirectory()?.appendingPathComponent("launcher-state.json") else { return }
        do {
            try FileManager.default.createDirectory(
                at: url.deletingLastPathComponent(),
                withIntermediateDirectories: true
            )
            let encoder = JSONEncoder()
            encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
            try encoder.encode(snapshot).write(to: url, options: .atomic)
        } catch {
            CrashLogRecording.record("launcher.state.export.failed error=\(error.localizedDescription)")
        }
    }

    private static func recordViolations(for snapshot: LauncherStateSnapshot) {
        let violations = LauncherStateInvariantChecker.check(snapshot)
        guard !violations.isEmpty else { return }

        for invariant in violations {
            let message = "launcher.state.violation \(invariant.rawValue)"
            CrashLogRecording.record(message)
        }

        guard isQAEnabled else { return }
        appendViolations(
            violations.map {
                LauncherStateViolation(
                    timestamp: snapshot.generatedAt,
                    invariant: $0,
                    snapshot: snapshot
                )
            }
        )
    }

    private static func appendViolations(_ newEntries: [LauncherStateViolation]) {
        guard let directory = logsDirectory() else { return }
        let url = directory.appendingPathComponent("launcher-state-violations.json")
        var existing: [LauncherStateViolation] = []
        if let data = try? Data(contentsOf: url),
           let decoded = try? JSONDecoder().decode([LauncherStateViolation].self, from: data) {
            existing = decoded
        }
        existing.append(contentsOf: newEntries)
        let trimmed = Array(existing.suffix(200))
        do {
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            let encoder = JSONEncoder()
            encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
            try encoder.encode(trimmed).write(to: url, options: .atomic)
        } catch {
            CrashLogRecording.record("launcher.state.violations.failed error=\(error.localizedDescription)")
        }
    }

    private static func logsDirectory() -> URL? {
        FileManager.default.urls(for: .libraryDirectory, in: .userDomainMask).first?
            .appendingPathComponent("Logs/Luma", isDirectory: true)
    }
}
