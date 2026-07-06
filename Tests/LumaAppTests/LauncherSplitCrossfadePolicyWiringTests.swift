import Foundation
import LumaCore
import Testing

@Test func splitCrossfadePolicyDuringAnimationDisablesHitTesting() {
    let during = LauncherSplitCrossfadePolicy.duringAnimation(.detailToGuide)
    #expect(during.guide.acceptsMouseHits == false)
    #expect(during.detail.acceptsMouseHits == false)
    let after = LauncherSplitCrossfadePolicy.afterAnimation(.detailToGuide)
    #expect(after.guide.acceptsMouseHits == true)
    #expect(after.detail.acceptsMouseHits == false)
}

@Test func homeSplitLayoutUsesCrossfadePolicy() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let path = root.appending(path: "Sources/LumaApp/Launcher/LauncherHomeSplitLayout.swift")
    let source = try String(contentsOf: path, encoding: .utf8)
    #expect(source.contains("LauncherSplitCrossfadePolicy.duringAnimation"))
    #expect(source.contains("LauncherSplitCrossfadePolicy.afterAnimation"))
    #expect(source.contains("crossfadeGeneration"))
    #expect(source.contains("invalidateCrossfadeCompletions"))
}

@Test func hideInvalidatesCrossfadeCompletionsInRootController() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let path = root.appending(path: "Sources/LumaApp/Launcher/LauncherRootController.swift")
    let source = try String(contentsOf: path, encoding: .utf8)
    #expect(source.contains("invalidateCrossfadeCompletions"))
}
