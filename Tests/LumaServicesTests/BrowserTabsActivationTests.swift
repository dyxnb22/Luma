import Testing
import LumaServices

@Test func safariActivationScriptBindsWindowBeforeReorder() {
    let record = TabRecord(
        bundleID: "com.apple.Safari",
        browserName: "Safari",
        windowIndex: 2,
        tabIndex: 3,
        title: "GitHub",
        url: "https://github.com"
    )
    let script = BrowserTabActivationScripts.safari(record: record)
    #expect(script.contains("set targetWindow to window 2"))
    #expect(script.contains("set index of targetWindow to 1"))
    #expect(script.contains("tell targetWindow to set current tab to tab 3"))
    #expect(!script.contains("tell window 2"))
}

@Test func chromiumActivationScriptBindsWindowBeforeReorder() {
    let record = TabRecord(
        bundleID: "com.google.Chrome",
        browserName: "Google Chrome",
        windowIndex: 4,
        tabIndex: 2,
        title: "Docs",
        url: "https://example.com"
    )
    let script = BrowserTabActivationScripts.chromium(applicationName: "Google Chrome", record: record)
    #expect(script.contains("set targetWindow to window 4"))
    #expect(script.contains("set index of targetWindow to 1"))
    #expect(script.contains("tell targetWindow to set active tab index to 2"))
    #expect(!script.contains("tell window 4"))
}
