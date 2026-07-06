import LumaCore
import Testing

@Test func beginHideIsNoOpWhenAlreadyHidden() {
    var session = LauncherPanelVisibilitySession()
    #expect(session.beginHide() == nil)
    #expect(!session.isVisible)
}

@Test func deferredShowWorkRequiresVisibleMatchingGeneration() {
    var session = LauncherPanelVisibilitySession()
    let generation = session.beginShow()
    #expect(session.shouldCompleteDeferredShow(generation: generation))
    _ = session.beginHide()
    #expect(!session.shouldCompleteDeferredShow(generation: generation))
}

@Test func rapidShowHideShowDiscardsStaleFinishHide() {
    var session = LauncherPanelVisibilitySession()
    _ = session.beginShow()
    let hideGeneration = session.beginHide()!
    let reshownGeneration = session.beginShow()
    #expect(!session.shouldCompleteHide(generationAtHide: hideGeneration))
    #expect(session.shouldCompleteDeferredShow(generation: reshownGeneration))
    #expect(session.isVisible)
}

@Test func hideDuringInFlightHideCompletionIsSupersededByReshow() {
    var session = LauncherPanelVisibilitySession()
    let showGeneration = session.beginShow()
    let firstHideGeneration = session.beginHide()!
    #expect(session.shouldCompleteHide(generationAtHide: firstHideGeneration))
    let secondShowGeneration = session.beginShow()
    #expect(!session.shouldCompleteHide(generationAtHide: firstHideGeneration))
    #expect(!session.shouldCompleteDeferredShow(generation: showGeneration))
    #expect(session.shouldCompleteDeferredShow(generation: secondShowGeneration))
}

@Test func finishHideCompletesWhenNoInterveningToggle() async {
    var session = LauncherPanelVisibilitySession()
    _ = session.beginShow()
    let hideGeneration = session.beginHide()!
    try? await Task.sleep(for: .milliseconds(1))
    #expect(session.shouldCompleteHide(generationAtHide: hideGeneration))
}
