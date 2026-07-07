import Foundation
import Testing
import LumaCore

@Test func backgroundCacheWarmNeverRepaintsOrAdvancesGeneration() {
    let guards = LauncherHomeRefreshRepaintPolicy.VisibleRepaintGuards(
        queryTrimmedEmpty: true,
        showingDetail: false,
        showingResults: false,
        isLauncherQueryEmpty: true
    )
    #expect(!LauncherHomeRefreshRepaintPolicy.shouldRepaintHome(intent: .backgroundCacheWarm, guards: guards))
    #expect(!LauncherHomeRefreshRepaintPolicy.shouldAdvanceRenderedGeneration(intent: .backgroundCacheWarm, didRepaint: true))
    #expect(!LauncherHomeRefreshRepaintPolicy.shouldAdvanceRenderedGeneration(intent: .backgroundCacheWarm, didRepaint: false))
    #expect(!LauncherHomeRefreshRepaintPolicy.shouldCloseHotkeyLatencyOnCacheHit(
        intent: .backgroundCacheWarm,
        isPanelActiveForQueryApply: true
    ))
}

@Test func visibleRepaintSkipsWhenQueryNonempty() {
    let guards = LauncherHomeRefreshRepaintPolicy.VisibleRepaintGuards(
        queryTrimmedEmpty: false,
        showingDetail: false,
        showingResults: false,
        isLauncherQueryEmpty: false
    )
    #expect(!LauncherHomeRefreshRepaintPolicy.shouldRepaintHome(intent: .visibleRepaint, guards: guards))
    #expect(!LauncherHomeRefreshRepaintPolicy.shouldAdvanceRenderedGeneration(intent: .visibleRepaint, didRepaint: false))
}

@Test func visibleRepaintSkipsWhenShowingResults() {
    let guards = LauncherHomeRefreshRepaintPolicy.VisibleRepaintGuards(
        queryTrimmedEmpty: true,
        showingDetail: false,
        showingResults: true,
        isLauncherQueryEmpty: true
    )
    #expect(!LauncherHomeRefreshRepaintPolicy.shouldRepaintHome(intent: .visibleRepaint, guards: guards))
}

@Test func visibleRepaintSkipsWhenQueryEmptyMirrorFalse() {
    let guards = LauncherHomeRefreshRepaintPolicy.VisibleRepaintGuards(
        queryTrimmedEmpty: true,
        showingDetail: false,
        showingResults: false,
        isLauncherQueryEmpty: false
    )
    #expect(!LauncherHomeRefreshRepaintPolicy.shouldRepaintHome(intent: .visibleRepaint, guards: guards))
}

@Test func visibleRepaintPaintsEmptyQueryHome() {
    let guards = LauncherHomeRefreshRepaintPolicy.VisibleRepaintGuards(
        queryTrimmedEmpty: true,
        showingDetail: false,
        showingResults: false,
        isLauncherQueryEmpty: true
    )
    #expect(LauncherHomeRefreshRepaintPolicy.shouldRepaintHome(intent: .visibleRepaint, guards: guards))
    #expect(LauncherHomeRefreshRepaintPolicy.shouldAdvanceRenderedGeneration(intent: .visibleRepaint, didRepaint: true))
    #expect(!LauncherHomeRefreshRepaintPolicy.shouldAdvanceRenderedGeneration(intent: .visibleRepaint, didRepaint: false))
}

@Test func visibleRepaintPaintsHomeColumnWhileDetailOpen() {
    let guards = LauncherHomeRefreshRepaintPolicy.VisibleRepaintGuards(
        queryTrimmedEmpty: true,
        showingDetail: true,
        showingResults: false,
        isLauncherQueryEmpty: true
    )
    #expect(LauncherHomeRefreshRepaintPolicy.shouldRepaintHome(intent: .visibleRepaint, guards: guards))
    #expect(LauncherHomeRefreshRepaintPolicy.shouldAdvanceRenderedGeneration(intent: .visibleRepaint, didRepaint: true))
}

@Test func cacheHitClosesHotkeyLatencyOnlyForVisibleRepaintWhenPanelActive() {
    #expect(LauncherHomeRefreshRepaintPolicy.shouldCloseHotkeyLatencyOnCacheHit(
        intent: .visibleRepaint,
        isPanelActiveForQueryApply: true
    ))
    #expect(!LauncherHomeRefreshRepaintPolicy.shouldCloseHotkeyLatencyOnCacheHit(
        intent: .visibleRepaint,
        isPanelActiveForQueryApply: false
    ))
    #expect(!LauncherHomeRefreshRepaintPolicy.shouldCloseHotkeyLatencyOnCacheHit(
        intent: .backgroundCacheWarm,
        isPanelActiveForQueryApply: true
    ))
}
