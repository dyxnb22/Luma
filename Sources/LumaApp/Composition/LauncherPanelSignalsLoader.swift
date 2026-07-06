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

    func load(includeSelection: Bool = true) async -> LauncherPanelSignals {
        if let cached = await PanelSignalsCache.shared.snapshot() {
            if includeSelection || cached.selectionText == nil {
                return cached
            }
        }

        let enabled = await config.enabledModules()
            ?? Set(ModuleRegistry.allBundles.map { $0.identifier })
        let pinned = await config.pinnedModuleIDs()
        async let clipboard = ClipboardPasteboardCache.shared.snapshot()
        let selection: String?
        if includeSelection {
            selection = await SelectionSnapshotService.shared.snapshot()
        } else {
            selection = await PanelSignalsCache.shared.snapshot()?.selectionText
        }
        let signals = LauncherPanelSignals(
            enabledModuleIDs: enabled,
            pinnedModuleIDs: pinned,
            clipboardPreview: await clipboard,
            selectionText: selection
        )
        await PanelSignalsCache.shared.store(signals)
        return signals
    }

    func loadWorkbenchContext(includeSelection: Bool = true) async -> WorkbenchContext {
        let signals = await load(includeSelection: includeSelection)
        return await WorkbenchContextBuilder().build(
            enabledModuleIDs: signals.enabledModuleIDs,
            pinnedModuleIDs: signals.pinnedModuleIDs,
            clipboardPreview: signals.clipboardPreview,
            selectionText: signals.selectionText
        )
    }

    func invalidateCache() async {
        await PanelSignalsCache.shared.invalidate()
    }
}
