import AppKit
import LumaCore

@MainActor
struct RecentDashboardItem {
    let title: String
    let subtitle: String
    let icon: NSImage
    let open: () -> Void
}

@MainActor
enum RecentItemProvider {
    static func recentItems(from results: [ResultItem], run: @escaping (ResultItem) -> Void) -> [RecentDashboardItem] {
        results.map { item in
            RecentDashboardItem(
                title: item.title,
                subtitle: item.subtitle ?? "Recent",
                icon: iconImage(for: item.icon),
                open: { run(item) }
            )
        }
    }

    private static func iconImage(for icon: LumaCore.IconRef) -> NSImage {
        switch icon {
        case .bundleID(let bundleID):
            if let url = NSWorkspace.shared.urlForApplication(withBundleIdentifier: bundleID) {
                return IconCache.shared.appIcon(for: url)
            }
            return NSImage(systemSymbolName: "app.fill", accessibilityDescription: nil) ?? NSImage()
        case .symbol(let symbol):
            return NSImage(systemSymbolName: symbol, accessibilityDescription: nil) ?? NSImage()
        case .file(let url):
            return IconCache.shared.appIcon(for: url)
        case .none:
            return NSImage(systemSymbolName: "sparkles", accessibilityDescription: nil) ?? NSImage()
        }
    }
}
