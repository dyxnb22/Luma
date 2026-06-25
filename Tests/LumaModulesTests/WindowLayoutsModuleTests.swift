import LumaCore
import LumaServices
import Testing
@testable import LumaModules

@Test func windowLayoutsModuleListsAllPresets() async {
    let module = WindowLayoutsModule()
    let result = await module.handle(Query(raw: "layout", sequence: 0), context: QueryContext(deadline: .now))
    if AXService.isProcessTrusted() {
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
    let result = await module.handle(Query(raw: "layout left", sequence: 0), context: QueryContext(deadline: .now))
    #expect(result.items.count == 1)
    #expect(result.items.first?.title == "Left Half")
    #expect(result.items.first?.primaryAction.kind == .applyWindowLayout("left-half"))
}

@Test func windowLayoutsModuleAcceptsWinAlias() async {
    guard AXService.isProcessTrusted() else { return }
    let module = WindowLayoutsModule()
    let result = await module.handle(Query(raw: "win center", sequence: 0), context: QueryContext(deadline: .now))
    #expect(result.items.count == 1)
    #expect(result.items.first?.title == "Center")
    #expect(result.items.first?.primaryAction.kind == .applyWindowLayout("center"))
}

@Test func windowLayoutsModuleDoesNotMatchUnrelatedQueries() async {
    let module = WindowLayoutsModule()
    let result = await module.handle(Query(raw: "left pad", sequence: 0), context: QueryContext(deadline: .now))
    #expect(result.items.isEmpty)
}

@Test func windowLayoutsExtractPayloadHandlesAliases() {
    #expect(WindowLayoutsModule.extractPayload(raw: "layout") == "")
    #expect(WindowLayoutsModule.extractPayload(raw: "layout left") == "left")
    #expect(WindowLayoutsModule.extractPayload(raw: "win right") == "right")
    #expect(WindowLayoutsModule.extractPayload(raw: "wl max") == "max")
    #expect(WindowLayoutsModule.extractPayload(raw: "safari") == nil)
}
