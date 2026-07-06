import AppKit
import Foundation
import LumaCore
import Testing
@testable import LumaApp

@MainActor
private final class MockModuleDetail: NSObject, ModuleDetailView {
    let detailView = NSView()
    let moduleTitle = "Mock"
    let usesSharedTopBar = false
    private(set) var activateCount = 0
    var detailContentGeneration: UInt64 = 1

    func refreshDetailContentGeneration() async {}

    func activate() {
        activateCount += 1
    }

    func deactivate() {}
}

@Test @MainActor func detailRegistrySkipsRedundantActivationWhenGenerationUnchanged() async {
    let registry = ModuleDetailRegistry()
    let detail = MockModuleDetail()

    await registry.activateDetailView(detail, moduleID: .clipboard)
    await registry.activateDetailView(detail, moduleID: .clipboard)

    #expect(detail.activateCount == 1)
}

@Test @MainActor func contentCoordinatorHidesPooledDetailViewOnClose() {
    let listView = LauncherListView(frame: .zero)
    let detailContainer = LauncherOverlayHostView()
    let detailTopBar = NSView()
    let detailTitleLabel = NSTextField(labelWithString: "")
    let contentContainer = NSView()
    let coordinator = LauncherContentCoordinator(
        listView: listView,
        detailContainer: detailContainer,
        detailTopBar: detailTopBar,
        detailTitleLabel: detailTitleLabel,
        contentContainer: contentContainer
    )

    let detail = MockModuleDetail()
    coordinator.present(detail, moduleID: .clipboard)
    coordinator.closeDetail()

    #expect(detail.detailView.isHidden)
    #expect(detail.detailView.superview === detailContainer)
}

@Test @MainActor func contentCoordinatorReusesPooledDetailViewOnSecondPresent() {
    let listView = LauncherListView(frame: .zero)
    let detailContainer = LauncherOverlayHostView()
    let detailTopBar = NSView()
    let detailTitleLabel = NSTextField(labelWithString: "")
    let contentContainer = NSView()
    let coordinator = LauncherContentCoordinator(
        listView: listView,
        detailContainer: detailContainer,
        detailTopBar: detailTopBar,
        detailTitleLabel: detailTitleLabel,
        contentContainer: contentContainer
    )

    let detail = MockModuleDetail()

    func layoutConstraintCount(for view: NSView) -> Int {
        detailContainer.constraints.filter {
            ($0.firstItem as? NSView) === view || ($0.secondItem as? NSView) === view
        }.count
    }

    coordinator.present(detail, moduleID: .clipboard)
    let constraintCountAfterFirstPresent = layoutConstraintCount(for: detail.detailView)
    #expect(constraintCountAfterFirstPresent == 4)

    coordinator.closeDetail()
    #expect(detail.detailView.isHidden)

    coordinator.present(detail, moduleID: .clipboard)
    #expect(!detail.detailView.isHidden)
    #expect(detail.detailView.superview === detailContainer)
    #expect(detailContainer.subviews.filter { $0 === detail.detailView }.count == 1)
    #expect(layoutConstraintCount(for: detail.detailView) == constraintCountAfterFirstPresent)
}
