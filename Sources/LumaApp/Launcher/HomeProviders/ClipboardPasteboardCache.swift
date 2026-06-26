import AppKit
import Foundation
import LumaCore
import LumaModules
import LumaServices

actor ClipboardPasteboardCache {
    static let shared = ClipboardPasteboardCache()

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
        let lumaBundleID = Bundle.main.bundleIdentifier
        while !Task.isCancelled {
            let preview = await MainActor.run {
                let snapshot = ClipboardSnapshotReader.read(lumaBundleID: lumaBundleID)
                return ClipboardFilter.homePreviewText(from: snapshot)
            }
            cachedValue = preview
            try? await Task.sleep(for: .seconds(refreshInterval))
        }
    }
}
