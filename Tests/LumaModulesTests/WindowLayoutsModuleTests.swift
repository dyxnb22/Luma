import LumaCore
import LumaServices
import Testing
@testable import LumaModules

private struct TrustedAccessibilityClient: AccessibilityClient {
    func isTrusted() async -> Bool { true }
    func requestPermission() async {}
    func focus(windowID: UInt32, pid: Int32, title: String, axTitle: String?, bounds: WindowBounds?) async {}
    func insert(text: String) async {}
    func applyWindowLayout(_ preset: String) async {}
}

private func queryContext(trusted: Bool) -> QueryContext {
    QueryContext(
        deadline: .now,
        platform: QueryPlatformClients(
            accessibility: trusted ? TrustedAccessibilityClient() : NoopAccessibilityClient()
        )
    )
}

@Test func windowLayoutsModuleListsAllPresets() async {
    let module = WindowLayoutsModule()
    let trusted = AXService.isProcessTrusted()
    let result = await module.handle(
        Query(raw: "layout", sequence: 0),
        context: queryContext(trusted: trusted)
    )
    if trusted {
        #expect(result.items.count == 6)
        let titles = Set(result.items.map(\.title))
        #expect(titles.contains("Left Half"))
        #expect(titles.contains("Center"))
    } else {
        #expect(result.items.count == 1)
        #expect(result.items.first?.title == "Grant Accessibility Permission")
    }
}

@Test func windowLayoutsModuleFiltersByPayload() async {
    guard AXService.isProcessTrusted() else { return }
    let module = WindowLayoutsModule()
    let result = await module.handle(
        Query(raw: "layout left", sequence: 0),
        context: queryContext(trusted: true)
    )
    #expect(result.items.count == 1)
    #expect(result.items.first?.title == "Left Half")
    #expect(result.items.first?.primaryAction.kind == .applyWindowLayout("left-half"))
}

@Test func windowLayoutsModuleAcceptsWinAlias() async {
    guard AXService.isProcessTrusted() else { return }
    let module = WindowLayoutsModule()
    let result = await module.handle(
        Query(raw: "win center", sequence: 0),
        context: queryContext(trusted: true)
    )
    #expect(result.items.count == 1)
    #expect(result.items.first?.title == "Center")
    #expect(result.items.first?.primaryAction.kind == .applyWindowLayout("center"))
}

@Test func windowLayoutsModuleDoesNotMatchUnrelatedQueries() async {
    let module = WindowLayoutsModule()
    let result = await module.handle(
        Query(raw: "left pad", sequence: 0),
        context: queryContext(trusted: true)
    )
    #expect(result.items.isEmpty)
}

@Test func windowLayoutsModuleExtractPayloadHandlesAliases() {
    #expect(WindowLayoutsModule.extractPayload(raw: "layout") == "")
    #expect(WindowLayoutsModule.extractPayload(raw: "layout left") == "left")
    #expect(WindowLayoutsModule.extractPayload(raw: "win right") == "right")
    #expect(WindowLayoutsModule.extractPayload(raw: "wl max") == "max")
    #expect(WindowLayoutsModule.extractPayload(raw: "safari") == nil)
}
