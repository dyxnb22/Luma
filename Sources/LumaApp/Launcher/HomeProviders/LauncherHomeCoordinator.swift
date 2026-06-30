import Foundation
import LumaCore
import LumaModules
import LumaServices

/// App-layer wiring for home list providers and open-apps expansion.
actor LauncherHomeCoordinator {
    private let aggregator: LauncherHomeAggregator
    private let openApps: OpenAppsHomeProvider
    private let enablementGate: HomeEnablementGate
    private var contextual: ContextualHomeProvider
    private let workbenchContextBuilder = WorkbenchContextBuilder()
    private var pinnedModuleIDs = ModuleWarmupDefaults.defaultPinnedModuleIDs
    private var showAllApps = false
    private var collapsedAppBundleIDs = Set<String>()

    init(
        openApps: OpenAppsHomeProvider,
        enablementGate: HomeEnablementGate,
        recentActions: RecentActionsHomeProvider = RecentActionsHomeProvider(),
        resume: ResumeHomeProvider,
        contextual: ContextualHomeProvider,
        setup: SetupHomeProvider? = nil
    ) {
        self.openApps = openApps
        self.enablementGate = enablementGate
        self.contextual = contextual
        self.aggregator = LauncherHomeAggregator(
            openApps: openApps,
            recentActions: recentActions,
            resume: resume,
            contextual: contextual,
            setup: setup
        )
    }

    func updatePinnedModuleIDs(_ ids: Set<ModuleIdentifier>) async {
        pinnedModuleIDs = ids
        await contextual.updatePinnedModuleIDs(ids)
    }

    func updateEnabledModuleIDs(_ ids: Set<ModuleIdentifier>) async {
        enablementGate.update(ids)
        await contextual.updateEnabledModuleIDs(ids)
    }

    func expandAllApps() {
        showAllApps = true
    }

    func resetExpansion() {
        showAllApps = false
        collapsedAppBundleIDs.removeAll()
    }

    func setActive(_ active: Bool) async {
        await openApps.setActive(active)
        await ClipboardPasteboardCache.shared.setActive(active)
        if active {
            _ = await CurrentProjectService.shared.snapshot()
            _ = await SelectionSnapshotService.shared.snapshot()
            await refreshWorkbenchContext()
        }
    }

    private func refreshWorkbenchContext() async {
        let clipboard = await ClipboardPasteboardCache.shared.snapshot()
        let selection = await SelectionSnapshotService.shared.snapshot()
        let enabled = enablementGate.snapshot() ?? Set(ModuleRegistry.allBundles.map { $0.identifier })
        let workbench = await workbenchContextBuilder.build(
            enabledModuleIDs: enabled,
            pinnedModuleIDs: pinnedModuleIDs,
            clipboardPreview: clipboard,
            selectionText: selection
        )
        await contextual.updateWorkbench(workbench)
    }

    func toggleAppWindows(bundleID: String) {
        if collapsedAppBundleIDs.contains(bundleID) {
            collapsedAppBundleIDs.remove(bundleID)
        } else {
            collapsedAppBundleIDs.insert(bundleID)
        }
    }

    func snapshot() async -> LauncherHomeSnapshot {
        await refreshWorkbenchContext()
        await openApps.configure(
            appLimit: showAllApps ? nil : OpenAppsHomeProvider.defaultAppLimit,
            collapsedBundleIDs: collapsedAppBundleIDs
        )
        return await aggregator.snapshot()
    }
}
