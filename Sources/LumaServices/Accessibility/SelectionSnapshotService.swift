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
            let text = await MainActor.run { Self.readSelectedText() }
            await self.store(text)
        }
        refreshTask = task
        await task.value
        refreshTask = nil
    }

    private func store(_ text: String?) {
        cachedText = text
        cachedAt = Date()
    }

    @MainActor
    private static func readSelectedText() -> String? {
        guard AXService.isProcessTrusted(),
              let app = NSWorkspace.shared.frontmostApplication else { return nil }
        let appElement = AXUIElementCreateApplication(app.processIdentifier)
        var focused: CFTypeRef?
        guard AXUIElementCopyAttributeValue(appElement, kAXFocusedUIElementAttribute as CFString, &focused) == .success,
              let element = focused else { return nil }
        var value: CFTypeRef?
        guard AXUIElementCopyAttributeValue(element as! AXUIElement, kAXSelectedTextAttribute as CFString, &value) == .success,
              let text = value as? String else { return nil }
        return text
    }
}
