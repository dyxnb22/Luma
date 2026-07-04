import Foundation
import Testing
import LumaCore

@Test func crossfadeDetailToGuideBlocksHitsDuringAnimation() {
    let state = LauncherSplitCrossfadePolicy.duringAnimation(.detailToGuide)
    #expect(!state.guide.acceptsMouseHits)
    #expect(!state.detail.acceptsMouseHits)
}

@Test func crossfadeDetailToGuideAllowsGuideHitsAfterCompletion() {
    let state = LauncherSplitCrossfadePolicy.afterAnimation(.detailToGuide)
    #expect(state.guide.acceptsMouseHits)
    #expect(!state.detail.acceptsMouseHits)
}

@Test func crossfadeGuideToDetailBlocksHitsDuringAnimation() {
    let state = LauncherSplitCrossfadePolicy.duringAnimation(.guideToDetail)
    #expect(!state.guide.acceptsMouseHits)
    #expect(!state.detail.acceptsMouseHits)
}

@Test func crossfadeGuideToDetailAllowsDetailHitsAfterCompletion() {
    let state = LauncherSplitCrossfadePolicy.afterAnimation(.guideToDetail)
    #expect(!state.guide.acceptsMouseHits)
    #expect(state.detail.acceptsMouseHits)
}
