import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices

/// Clipboard, selection, and module enablement snapshots for workbench command/capture paths.
struct LauncherPanelSignals: Sendable {
    let enabledModuleIDs: Set<ModuleIdentifier>
    let pinnedModuleIDs: Set<ModuleIdentifier>
    let clipboardPreview: String?
    let selectionText: String?
}

/// Loads panel signals used by `WorkbenchContextBuilder` and workbench executors.
struct LauncherPanelSignalsLoader {
    let config: ConfigurationStore

    func load() async -> LauncherPanelSignals {
        let enabled = await config.enabledModules()
            ?? Set(ModuleRegistry.allBundles.map { $0.identifier })
        let pinned = await config.pinnedModuleIDs()
        async let clipboard = ClipboardPasteboardCache.shared.snapshot()
        async let selection = SelectionSnapshotService.shared.snapshot()
        return LauncherPanelSignals(
            enabledModuleIDs: enabled,
            pinnedModuleIDs: pinned,
            clipboardPreview: await clipboard,
            selectionText: await selection
        )
    }

    func loadWorkbenchContext() async -> WorkbenchContext {
        let signals = await load()
        return await WorkbenchContextBuilder().build(
            enabledModuleIDs: signals.enabledModuleIDs,
            pinnedModuleIDs: signals.pinnedModuleIDs,
            clipboardPreview: signals.clipboardPreview,
            selectionText: signals.selectionText
        )
    }
}
