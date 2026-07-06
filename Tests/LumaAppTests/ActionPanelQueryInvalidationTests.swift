import Foundation
import LumaCore
import Testing

@Test func actionPanelDismissesWhenQueryChanges() {
    #expect(
        LauncherActionPanelInvalidationPolicy.shouldDismissOnQueryChange(
            previousQuery: "clip",
            newQuery: "clip x",
            actionPanelVisible: true
        )
    )
}

@Test func actionPanelStaysWhenQueryUnchanged() {
    #expect(
        !LauncherActionPanelInvalidationPolicy.shouldDismissOnQueryChange(
            previousQuery: "clip",
            newQuery: "clip",
            actionPanelVisible: true
        )
    )
}

@Test func actionPanelStaysWhenNotVisible() {
    #expect(
        !LauncherActionPanelInvalidationPolicy.shouldDismissOnQueryChange(
            previousQuery: "clip",
            newQuery: "clip x",
            actionPanelVisible: false
        )
    )
}
