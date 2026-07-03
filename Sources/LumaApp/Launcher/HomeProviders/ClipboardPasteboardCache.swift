import Foundation
import LumaCore
import LumaModules
import LumaServices

actor ClipboardPasteboardCache {
    static let shared = ClipboardPasteboardCache()

    private let snapshotService = ClipboardSnapshotService()
    private var cachedValue: String?
    private var refreshTask: Task<Void, Never>?
    private var isActive = false
    private let refreshInterval: TimeInterval = 1.5

    func snapshot() -> String? {
        cachedValue
    }

    func setActive(_ active: Bool) {
        guard isActive != active else { return }
        isActive = active
        if active {
            refreshTask?.cancel()
            refreshTask = Task { [weak self] in await self?.refreshLoop() }
        } else {
            refreshTask?.cancel()
            refreshTask = nil
        }
    }

    private func refreshLoop() async {
        while !Task.isCancelled {
            let snapshot = await snapshotService.readSnapshot()
            cachedValue = ClipboardFilter.clipboardPreviewText(from: snapshot)
            try? await Task.sleep(for: .seconds(refreshInterval))
        }
    }
}
