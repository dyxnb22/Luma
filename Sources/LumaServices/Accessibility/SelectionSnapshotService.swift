import AppKit
import ApplicationServices
import Foundation

public actor SelectionSnapshotService {
    public static let shared = SelectionSnapshotService()

    private var cachedText: String?
    private var cachedAt: Date?
    private var refreshTask: Task<Void, Never>?
    private let ttl: TimeInterval = 1.5

    public init() {}

    public func snapshot() async -> String? {
        await refreshIfNeeded()
        return cachedText
    }

    public func refreshIfNeeded() async {
        if let cachedAt, Date().timeIntervalSince(cachedAt) < ttl { return }
        if let refreshTask {
            await refreshTask.value
            return
        }

        let task = Task {
            // Capture PID on MainActor (fast AppKit property access), then do
            // the synchronous AX IPC calls on this background task to avoid
            // blocking the main thread for the full cross-process round-trip.
            let pid = await MainActor.run { NSWorkspace.shared.frontmostApplication?.processIdentifier }
            let text = pid.flatMap { Self.readSelectedText(pid: $0) }
            await self.store(text)
        }
        refreshTask = task
        await task.value
        refreshTask = nil
    }

    private func store(_ text: String?) async {
        cachedText = text
        cachedAt = Date()
    }

    private static func readSelectedText(pid: pid_t) -> String? {
        guard AXService.isProcessTrusted() else { return nil }
        let appElement = AXUIElementCreateApplication(pid)
        var focused: CFTypeRef?
        guard AXUIElementCopyAttributeValue(appElement, kAXFocusedUIElementAttribute as CFString, &focused) == .success,
              let element = focused else { return nil }
        var value: CFTypeRef?
        guard AXUIElementCopyAttributeValue(element as! AXUIElement, kAXSelectedTextAttribute as CFString, &value) == .success,
              let text = value as? String else { return nil }
        return text
    }
}
