import AppKit

@MainActor
final class IconCache {
    static let shared = IconCache()

    private let cache = NSCache<NSString, NSImage>()

    private init(limit: Int = 96) {
        cache.countLimit = limit
    }

    func appIcon(for url: URL) -> NSImage {
        let key = url.path as NSString
        if let cached = cache.object(forKey: key) {
            return cached
        }
        let icon = NSWorkspace.shared.icon(forFile: url.path)
        cache.setObject(icon, forKey: key)
        return icon
    }

    func runningAppIcon(_ app: NSRunningApplication) -> NSImage {
        let key = (app.bundleIdentifier ?? app.localizedName ?? "\(app.processIdentifier)") as NSString
        if let cached = cache.object(forKey: key) {
            return cached
        }
        let icon = app.icon ?? NSImage()
        cache.setObject(icon, forKey: key)
        return icon
    }

    func trimForMemoryPressure() {
        cache.removeAllObjects()
        cache.countLimit = max(32, cache.countLimit / 2)
    }
}
