import AppKit
import LumaWorkbenchCore

protocol LumaWindowControllerDelegate: AnyObject {
    /// The close button was pressed. The host hides instead of terminating.
    func windowControllerDidRequestHide(_ controller: LumaWindowController)
}

/// The one workbench window. There is never a second one, and it holds exactly one content view:
/// the terminal.
final class LumaWindowController: NSObject, NSWindowDelegate {
    let window: NSWindow
    private let terminalView: NSView

    weak var delegate: LumaWindowControllerDelegate?

    var isVisible: Bool { window.isVisible }

    init(terminalView: NSView, cellSize: CGSize) {
        self.terminalView = terminalView

        let contentSize = TerminalGeometry.integralContentSize(
            preferred: TerminalGeometry.preferredContentSize,
            cell: cellSize
        )
        window = NSWindow(
            contentRect: NSRect(origin: .zero, size: contentSize),
            styleMask: [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView],
            backing: .buffered,
            defer: false
        )
        super.init()

        window.title = HostIdentity.applicationName
        window.titleVisibility = .hidden
        window.titlebarAppearsTransparent = true
        window.backgroundColor = TerminalSessionController.backgroundColor
        window.appearance = NSAppearance(named: .darkAqua)
        window.isReleasedWhenClosed = false
        window.tabbingMode = .disallowed
        // Summoning must not drag the user to another desktop, and the window must not sit on top
        // of everything the rest of the time.
        // `fullScreenAuxiliary` lets the workbench join the current Space when the previous app is
        // full-screen. `fullScreenPrimary` would instead only allow Luma to create its own Space.
        window.collectionBehavior = [.moveToActiveSpace, .fullScreenAuxiliary]
        window.level = .normal
        window.minSize = NSSize(
            width: CGFloat(TerminalGeometry.minimumColumns) * max(cellSize.width, 1),
            height: CGFloat(TerminalGeometry.minimumRows) * max(cellSize.height, 1)
        )
        window.delegate = self

        installTerminalView()

        // Centered on first launch, remembered afterwards.
        if !window.setFrameUsingName(HostIdentity.windowFrameAutosaveName) {
            window.center()
        }
        window.setFrameAutosaveName(HostIdentity.windowFrameAutosaveName)
    }

    private func installTerminalView() {
        guard let contentView = window.contentView else { return }
        terminalView.translatesAutoresizingMaskIntoConstraints = false
        contentView.addSubview(terminalView)
        // `contentLayoutGuide` excludes the titlebar, so full-size content does not hide the top
        // row of the grid behind the traffic lights.
        guard let layoutGuide = window.contentLayoutGuide as? NSLayoutGuide else {
            NSLayoutConstraint.activate([
                terminalView.leadingAnchor.constraint(equalTo: contentView.leadingAnchor),
                terminalView.trailingAnchor.constraint(equalTo: contentView.trailingAnchor),
                terminalView.topAnchor.constraint(equalTo: contentView.topAnchor),
                terminalView.bottomAnchor.constraint(equalTo: contentView.bottomAnchor)
            ])
            return
        }
        NSLayoutConstraint.activate([
            terminalView.leadingAnchor.constraint(equalTo: layoutGuide.leadingAnchor),
            terminalView.trailingAnchor.constraint(equalTo: layoutGuide.trailingAnchor),
            terminalView.topAnchor.constraint(equalTo: layoutGuide.topAnchor),
            terminalView.bottomAnchor.constraint(equalTo: layoutGuide.bottomAnchor)
        ])
    }

    /// Shows the window on whichever Space is active and gives the terminal the keyboard.
    func showOnCurrentSpace() {
        if window.isMiniaturized {
            window.deminiaturize(nil)
        }
        window.makeKeyAndOrderFront(nil)
        window.makeFirstResponder(terminalView)
    }

    func hide() {
        window.orderOut(nil)
    }

    // MARK: - NSWindowDelegate

    func windowShouldClose(_ sender: NSWindow) -> Bool {
        delegate?.windowControllerDidRequestHide(self)
        return false
    }
}
