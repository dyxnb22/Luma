import AppKit

@MainActor
protocol ModuleDetailView: AnyObject {
    var detailView: NSView { get }
    var moduleTitle: String { get }
    var usesSharedTopBar: Bool { get }
    func activate()
    func deactivate()
    func handleKeyDown(_ event: NSEvent) -> Bool
    func prepareForLauncherHide() async
}

extension ModuleDetailView {
    var usesSharedTopBar: Bool { true }
    func handleKeyDown(_ event: NSEvent) -> Bool { false }
    func prepareForLauncherHide() async {}
}
