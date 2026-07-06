import Foundation
import LumaCore
import Testing
@testable import LumaApp

@Test func panelSignalsCacheHonorsTTL() async {
    await PanelSignalsCache.shared.invalidate()
    let signals = LauncherPanelSignals(
        enabledModuleIDs: [.apps],
        pinnedModuleIDs: [],
        clipboardPreview: "hello",
        selectionText: nil
    )
    await PanelSignalsCache.shared.store(signals)
    let hit = await PanelSignalsCache.shared.snapshot()
    #expect(hit?.clipboardPreview == "hello")
    await PanelSignalsCache.shared.invalidate()
    let miss = await PanelSignalsCache.shared.snapshot()
    #expect(miss == nil)
}
