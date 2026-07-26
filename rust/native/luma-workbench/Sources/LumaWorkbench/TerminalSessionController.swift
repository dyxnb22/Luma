import AppKit
import Darwin
import LumaWorkbenchCore
import SwiftTerm

protocol TerminalSessionControllerDelegate: AnyObject {
    func terminalSessionDidStart(_ controller: TerminalSessionController)
    func terminalSessionDidTerminate(_ controller: TerminalSessionController)
}

/// SwiftTerm hands LocalProcess output directly to its terminal parser. Luma's
/// policy layer sits immediately before that hand-off so remote terminal output
/// cannot use OSC/APC side channels to access the macOS pasteboard or retain an
/// unbounded string in the dependency parser.
private final class LumaTerminalView: LocalProcessTerminalView {
    private var controlFilter = TerminalControlFilter()

    override func dataReceived(slice: ArraySlice<UInt8>) {
        let safe = controlFilter.filter(slice)
        guard !safe.isEmpty else { return }
        super.dataReceived(slice: safe[...])
    }
}

/// Owns the single `luma tui` child process and the SwiftTerm view it draws into.
///
/// One session, one view, for the lifetime of the app: hiding the window never touches the child,
/// so TUI state, running Timers, and interactive children (ssh, sftp, recipes) survive.
final class TerminalSessionController: NSObject, LocalProcessTerminalViewDelegate {
    let terminalView: LocalProcessTerminalView

    private let executableURL: URL
    private let workingDirectory: String
    private var lifecycle = SessionLifecycle()

    weak var delegate: TerminalSessionControllerDelegate?

    var isRunning: Bool { lifecycle.isRunning }
    var processIdentifier: pid_t? {
        let pid = terminalView.process.shellPid
        return lifecycle.isRunning && pid > 1 ? pid : nil
    }

    init(executableURL: URL, workingDirectory: String) {
        self.executableURL = executableURL
        self.workingDirectory = workingDirectory
        self.terminalView = LumaTerminalView(
            frame: NSRect(origin: .zero, size: TerminalGeometry.preferredContentSize)
        )
        super.init()
        configureTerminalView()
        terminalView.processDelegate = self
    }

    /// Terminal background. The window borrows it so the titlebar does not cut a bright strip
    /// across the top; everything else on screen is drawn by Ratatui's own ANSI colors.
    static let backgroundColor = NSColor(srgbRed: 0.07, green: 0.07, blue: 0.08, alpha: 1)
    static let foregroundColor = NSColor(srgbRed: 0.87, green: 0.87, blue: 0.88, alpha: 1)

    private func configureTerminalView() {
        terminalView.font = TerminalSessionController.monospaceFont()
        terminalView.nativeForegroundColor = TerminalSessionController.foregroundColor
        terminalView.nativeBackgroundColor = TerminalSessionController.backgroundColor
        terminalView.caretColor = TerminalSessionController.foregroundColor
        // The Luma TUI binds Control shortcuts only and has no Meta bindings, so leaving Option as
        // a meta prefix would break Option-composed characters for no gain.
        terminalView.optionAsMetaKey = false
    }

    private static func monospaceFont(size: CGFloat = 13) -> NSFont {
        let system = NSFont.monospacedSystemFont(ofSize: size, weight: .regular)
        if system.isFixedPitch { return system }
        return NSFont(name: "Menlo", size: size) ?? system
    }

    /// Size of one character cell, derived from SwiftTerm's optimal frame for the current grid.
    /// SwiftTerm keeps `cellDimension` internal, so this is the supported way to ask.
    func cellSize() -> CGSize {
        let optimal = terminalView.getOptimalFrameSize()
        let columns = CGFloat(max(1, terminalView.terminal.cols))
        let rows = CGFloat(max(1, terminalView.terminal.rows))
        return CGSize(width: optimal.width / columns, height: optimal.height / rows)
    }

    /// Starts a session if none is running. Called on every activation; a live child is left alone.
    @discardableResult
    func startIfNeeded() -> Bool {
        guard lifecycle.needsStartBeforeShowing else { return true }
        if case .exited = lifecycle.state {
            terminalView.terminal.resetToInitialState()
        }
        let environment = ChildEnvironment.make(
            inherited: ProcessInfo.processInfo.environment,
            homeDirectory: workingDirectory
        )
        terminalView.startProcess(
            executable: executableURL.path,
            args: BundledExecutable.tuiArguments,
            environment: environment,
            execName: nil,
            currentDirectory: workingDirectory
        )
        guard terminalView.process.running else {
            lifecycle.markTerminated(swiftTermWaitStatus: nil)
            terminalView.feed(text: "\r\n[could not start \(executableURL.path)]\r\n")
            return false
        }
        lifecycle.markStarted()
        delegate?.terminalSessionDidStart(self)
        return true
    }

    func applyMemoryPressure(_ level: MemoryPressureLevel) {
        terminalView.changeScrollback(MemoryPressurePolicy.scrollbackLines(for: level))
    }

    /// Ask the process group to terminate, then reap it. The Rust TUI handles SIGTERM through its
    /// normal event loop so ShutdownSession/module teardown runs; SIGKILL remains a bounded fallback.
    func terminateAndReap() {
        let pid = terminalView.process.shellPid
        guard lifecycle.isRunning, pid > 0 else { return }
        // Signal the forkpty process group so an interactive SSH/SFTP/recipe child exits and the
        // TUI can regain control long enough to finish graceful teardown.
        if let group = ChildProcessGroup.signalTarget(forLeader: pid) {
            _ = kill(group, SIGTERM)
        }
        reap(pid: pid)
        lifecycle.markTerminated(swiftTermWaitStatus: nil)
    }

    private func reap(pid: pid_t, timeout: TimeInterval = 3) {
        let deadline = Date().addingTimeInterval(timeout)
        var status: Int32 = 0
        while Date() < deadline {
            let result = waitpid(pid, &status, WNOHANG)
            // 0 means still running; anything else means reaped, or already reaped by SwiftTerm's
            // process monitor (ECHILD).
            if result != 0 { return }
            usleep(20_000)
        }
        if let group = ChildProcessGroup.signalTarget(forLeader: pid) {
            _ = kill(group, SIGKILL)
        } else {
            _ = kill(pid, SIGKILL)
        }
        _ = waitpid(pid, &status, 0)
    }

    // MARK: - LocalProcessTerminalViewDelegate

    func sizeChanged(source: LocalProcessTerminalView, newCols: Int, newRows: Int) {}

    func setTerminalTitle(source: LocalProcessTerminalView, title: String) {}

    func hostCurrentDirectoryUpdate(source: TerminalView, directory: String?) {}

    func processTerminated(source: TerminalView, exitCode: Int32?) {
        lifecycle.markTerminated(swiftTermWaitStatus: exitCode)
        if let notice = lifecycle.terminationNotice {
            terminalView.feed(text: "\r\n\(notice)\r\n")
        }
        delegate?.terminalSessionDidTerminate(self)
    }
}
