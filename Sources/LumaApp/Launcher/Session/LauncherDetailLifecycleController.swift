import AppKit
import LumaCore
import LumaModules

@MainActor
final class LauncherDetailLifecycleController {
    private let contentCoordinator: LauncherContentCoordinator
    private let homeSplitLayout: LauncherHomeSplitLayout
    private let searchBar: LumaSearchBar
    private let usesColumnSplitLayout: () -> Bool
    private let discoverableCommands: () -> [CommandDefinition]
    private let enabledModuleIDs: () -> Set<ModuleIdentifier>
    private var detailCloseCrossfadeInFlight = false
    private var detailPresentationGeneration = CancellationGeneration()

    var onTearDown: (() -> Void)?
    var onAfterClose: (() -> Void)?

    init(
        contentCoordinator: LauncherContentCoordinator,
        homeSplitLayout: LauncherHomeSplitLayout,
        searchBar: LumaSearchBar,
        usesColumnSplitLayout: @escaping () -> Bool,
        discoverableCommands: @escaping () -> [CommandDefinition],
        enabledModuleIDs: @escaping () -> Set<ModuleIdentifier>
    ) {
        self.contentCoordinator = contentCoordinator
        self.homeSplitLayout = homeSplitLayout
        self.searchBar = searchBar
        self.usesColumnSplitLayout = usesColumnSplitLayout
        self.discoverableCommands = discoverableCommands
        self.enabledModuleIDs = enabledModuleIDs
    }

    var isShowingDetail: Bool { contentCoordinator.showingDetail }

    func invalidateCrossfadeCompletions() {
        homeSplitLayout.invalidateCrossfadeCompletions()
    }

    func cancelPendingPresentation() {
        detailPresentationGeneration.bump()
    }

    func consumeDetailCloseCrossfadeInFlight() -> Bool {
        let pending = detailCloseCrossfadeInFlight && contentCoordinator.showingDetail
        detailCloseCrossfadeInFlight = false
        return pending
    }

    func closeDetail(animatedToGuide: Bool = false, completion: (@MainActor () -> Void)? = nil) {
        homeSplitLayout.invalidateCrossfadeCompletions()
        guard contentCoordinator.showingDetail else {
            completion?()
            return
        }
        if animatedToGuide, usesColumnSplitLayout() {
            let enabled = enabledModuleIDs()
            let commands = discoverableCommands().filter { enabled.contains($0.module) }
            detailCloseCrossfadeInFlight = true
            homeSplitLayout.crossfadeFromDetailToGuide(
                commands: commands,
                enabledModules: enabled
            ) { [weak self] in
                self?.detailCloseCrossfadeInFlight = false
                self?.tearDownAfterGuideCrossfade()
                completion?()
            }
        } else {
            tearDownAfterGuideCrossfade()
            completion?()
        }
    }

    func tearDownAfterGuideCrossfadeIfNeeded() {
        tearDownAfterGuideCrossfade()
    }

    private func tearDownAfterGuideCrossfade() {
        contentCoordinator.closeDetail(presentation: .rightColumn)
        searchBar.clearStuckDetailModeState()
        onTearDown?()
        onAfterClose?()
    }

    func nextPresentationGeneration() -> UInt {
        detailPresentationGeneration.bump()
        return detailPresentationGeneration.current
    }

    func isPresentationGenerationCurrent(_ generation: UInt) -> Bool {
        detailPresentationGeneration.isCurrent(generation)
    }

    func currentPresentationGeneration() -> UInt {
        detailPresentationGeneration.current
    }

    func bumpPresentationGeneration() {
        detailPresentationGeneration.bump()
    }
}
