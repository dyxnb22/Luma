import Foundation
import Testing
import LumaCore

@Test func usageResultCacheSkipsSensitiveModules() async throws {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent("usage-cache-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url) }

    let cache = UsageResultCache(url: url)
    let clipboardItem = ResultItem(
        id: ResultID(module: ModuleIdentifier(rawValue: "luma.clipboard"), key: "clip.1"),
        title: "Clipboard",
        titleAttributed: AttributedString("Clipboard"),
        subtitle: "secret-token",
        icon: .symbol("doc.on.clipboard"),
        primaryAction: Action(
            id: ActionID(module: ModuleIdentifier(rawValue: "luma.clipboard"), key: "copy"),
            title: "Copy",
            kind: .copyToPasteboard("secret-token")
        ),
        rankingHints: RankingHints()
    )
    let appsItem = ResultItem(
        id: ResultID(module: ModuleIdentifier(rawValue: "luma.apps"), key: "com.apple.Safari"),
        title: "Safari",
        titleAttributed: AttributedString("Safari"),
        icon: .bundleID("com.apple.Safari"),
        primaryAction: Action(
            id: ActionID(module: ModuleIdentifier(rawValue: "luma.apps"), key: "launch"),
            title: "Open",
            kind: .launchApp(URL(fileURLWithPath: "/Applications/Safari.app"))
        ),
        rankingHints: RankingHints()
    )

    await cache.store(clipboardItem)
    await cache.store(appsItem)

    #expect(await cache.item(for: clipboardItem.id) == nil)
    #expect(await cache.item(for: appsItem.id)?.title == "Safari")

    let data = try Data(contentsOf: url)
    let json = String(decoding: data, as: UTF8.self)
    #expect(!json.contains("secret-token"))
}

@Test func usageResultCachePurgesSensitiveEntriesOnLoad() async throws {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent("usage-cache-purge-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url) }

    let sensitive = StorableResultItem(
        ResultItem(
            id: ResultID(module: ModuleIdentifier(rawValue: "luma.secrets"), key: "entry.1"),
            title: "API Key",
            titleAttributed: AttributedString("API Key"),
            subtitle: "sk-live-secret",
            icon: .symbol("lock"),
            primaryAction: Action(
                id: ActionID(module: ModuleIdentifier(rawValue: "luma.secrets"), key: "copy"),
                title: "Copy",
                kind: .copyToPasteboard("sk-live-secret")
            ),
            rankingHints: RankingHints()
        )
    )
    let data = try JSONEncoder().encode([sensitive])
    try data.write(to: url)

    let cache = UsageResultCache(url: url)
    #expect(await cache.item(for: sensitive.id) == nil)
    let reloaded = try String(decoding: Data(contentsOf: url), as: UTF8.self)
    #expect(!reloaded.contains("sk-live-secret"))
}
