import AppKit

@MainActor
final class IconCache {
    static let shared = IconCache()

    private let cache = NSCache<NSString, NSImage>()
    private let bundleURLCache = NSCache<NSString, NSURL>()

    private init(limit: Int = 96) {
        cache.countLimit = limit
        bundleURLCache.countLimit = limit
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

    func appIcon(bundleID: String) -> NSImage {
        if let url = applicationURL(forBundleID: bundleID) {
            return appIcon(for: url)
        }
        return NSImage(systemSymbolName: "app.fill", accessibilityDescription: nil) ?? NSImage()
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
        bundleURLCache.removeAllObjects()
        cache.countLimit = max(32, cache.countLimit / 2)
        bundleURLCache.countLimit = max(32, bundleURLCache.countLimit / 2)
    }

    private func applicationURL(forBundleID bundleID: String) -> URL? {
        let key = bundleID as NSString
        if let cached = bundleURLCache.object(forKey: key) {
            return cached as URL
        }
        guard let url = NSWorkspace.shared.urlForApplication(withBundleIdentifier: bundleID) else {
            return nil
        }
        bundleURLCache.setObject(url as NSURL, forKey: key)
        return url
    }
}
