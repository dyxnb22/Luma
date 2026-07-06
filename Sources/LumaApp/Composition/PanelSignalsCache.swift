import Foundation
import LumaCore
import LumaModules

actor PanelSignalsCache {
    static let shared = PanelSignalsCache()

    private var cached: LauncherPanelSignals?
    private var cachedAt: ContinuousClock.Instant?
    private static let ttl: Duration = .seconds(2)

    func snapshot() -> LauncherPanelSignals? {
        guard let cached, let cachedAt else { return nil }
        if ContinuousClock.now - cachedAt > Self.ttl {
            return nil
        }
        return cached
    }

    func store(_ signals: LauncherPanelSignals) {
        cached = signals
        cachedAt = .now
    }

    func invalidate() {
        cached = nil
        cachedAt = nil
    }
}
