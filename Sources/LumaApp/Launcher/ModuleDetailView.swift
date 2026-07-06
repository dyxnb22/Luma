import AppKit

@MainActor
protocol ModuleDetailView: AnyObject {
    var detailView: NSView { get }
    var moduleTitle: String { get }
    var usesSharedTopBar: Bool { get }
    func refreshDetailContentGeneration() async
    var detailContentGeneration: UInt64 { get }
    func activate()
    func activate(generation: UInt64)
    func deactivate()
    func handleKeyDown(_ event: NSEvent) -> Bool
    func prepareForLauncherHide() async
}

extension ModuleDetailView {
    var usesSharedTopBar: Bool { true }
    var detailContentGeneration: UInt64 { 0 }
    func refreshDetailContentGeneration() async {}
    func activate(generation: UInt64) {
        activate()
    }
    func handleKeyDown(_ event: NSEvent) -> Bool {
        LumaStandardEditShortcuts.handleKeyDown(event, in: detailView.window)
    }
    func prepareForLauncherHide() async {}
}
