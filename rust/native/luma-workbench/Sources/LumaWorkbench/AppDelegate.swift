import AppKit
import LumaWorkbenchCore

/// Wires the four pieces of the host together: bundled binary → PTY session → window → hotkey.
///
/// Everything the user actually works with is drawn by the Rust TUI inside the terminal view.
final class AppDelegate: NSObject, NSApplicationDelegate {
    private static let activationShortcutDefaultsKey = "activationShortcut"
    private var session: TerminalSessionController?
    private var windowController: LumaWindowController?
    private var hotKeyController: GlobalHotKeyController?
    private var memoryPressureController: MemoryPressureController?
    private var activation = ActivationPolicy()
    private var pendingExternalShowRequest = false

    private var ownProcessIdentifier: pid_t { ProcessInfo.processInfo.processIdentifier }

    override init() {
        super.init()
        DistributedNotificationCenter.default().addObserver(
            self,
            selector: #selector(showRunningInstance),
            name: Notification.Name(HostIdentity.showRunningInstanceNotification),
            object: nil
        )
    }

    deinit {
        DistributedNotificationCenter.default().removeObserver(self)
    }

    // MARK: - NSApplicationDelegate

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.accessory)

        guard let executableURL = resolveBundledLuma() else { return }

        let session = TerminalSessionController(
            executableURL: executableURL,
            workingDirectory: NSHomeDirectory()
        )
        session.delegate = self
        self.session = session
        // Explicit edit targets keep Cmd-C/V/A and their menu items available even while AppKit
        // reports the accessory window, rather than SwiftTerm, as the responder between events.
        NSApp.mainMenu = AppMenu.make(editTarget: session.terminalView)
        let memoryPressureController = MemoryPressureController(session: session)
        memoryPressureController.start()
        self.memoryPressureController = memoryPressureController

        let windowController = LumaWindowController(
            terminalView: session.terminalView,
            cellSize: session.cellSize()
        )
        windowController.delegate = self
        self.windowController = windowController

        let savedHotKey = HotKeyDefinition.saved(
            identifier: UserDefaults.standard.string(
                forKey: Self.activationShortcutDefaultsKey
            )
        )
        registerHotKey(savedHotKey)

        // An activation request made while the app is still launching is dropped, which leaves the
        // window ordered behind whatever was frontmost. One turn of the run loop is enough.
        DispatchQueue.main.async { [weak self] in
            guard let self else { return }
            self.apply(
                self.activation.show(
                    frontmostApplication: nil,
                    ownProcessIdentifier: self.ownProcessIdentifier,
                    sessionIsRunning: false
                )
            )
            if self.pendingExternalShowRequest {
                self.pendingExternalShowRequest = false
                self.showWindow()
            }
        }
    }

    func applicationWillTerminate(_ notification: Notification) {
        hotKeyController?.unregister()
        memoryPressureController?.stopAndLogPeak()
        session?.terminateAndReap()
    }

    /// Hiding the only window must not quit the app; the session keeps running.
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        false
    }

    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows: Bool) -> Bool {
        showWindow()
        return true
    }

    // MARK: - Activation

    private func handleHotKey() {
        let frontmost = NSWorkspace.shared.frontmostApplication.map {
            PreviousApplication(
                processIdentifier: $0.processIdentifier,
                bundleIdentifier: $0.bundleIdentifier
            )
        }
        let action = activation.toggle(
            // A miniaturized window is not presented even if the accessory app remains active;
            // the hotkey must restore it rather than consume a hide transition first.
            applicationIsActive: NSApp.isActive
                && !(windowController?.window.isMiniaturized ?? false),
            frontmostApplication: frontmost,
            ownProcessIdentifier: ownProcessIdentifier,
            sessionIsRunning: session?.isRunning ?? false
        )
        apply(action)
    }

    private func showWindow() {
        let action = activation.show(
            frontmostApplication: nil,
            ownProcessIdentifier: ownProcessIdentifier,
            sessionIsRunning: session?.isRunning ?? false
        )
        apply(action)
    }

    @objc private func showRunningInstance() {
        guard windowController != nil else {
            pendingExternalShowRequest = true
            return
        }
        showWindow()
    }

    private func apply(_ action: ActivationAction) {
        switch action {
        case .show(let startSession):
            if startSession {
                session?.startIfNeeded()
            }
            activateApplication()
            windowController?.showOnCurrentSpace()
        case .hide(let previous):
            windowController?.hide()
            if let previous, reactivate(previous) { return }
            // Nothing to hand focus back to (cold launch, or the app is gone). Without this the
            // host stays frontmost with no window and keystrokes go nowhere.
            NSApp.hide(nil)
        }
    }

    /// macOS 14 replaced this with the cooperative `NSApp.activate()`, which silently does nothing
    /// when another application currently owns activation — precisely the case for an accessory app
    /// being summoned. Observed on macOS 26: cooperative activation leaves the window ordered
    /// behind the frontmost app on cold launch. The deprecated call is the one that works, so it is
    /// isolated here in a deprecated context rather than spread across the delegate.
    @available(macOS, deprecated: 14.0)
    private func activateApplication() {
        NSApp.activate(ignoringOtherApps: true)
    }

    /// Returns false when the remembered application is gone or its PID has been recycled.
    private func reactivate(_ previous: PreviousApplication) -> Bool {
        guard let application = NSRunningApplication(processIdentifier: previous.processIdentifier),
              !application.isTerminated
        else { return false }
        let current = PreviousApplication(
            processIdentifier: application.processIdentifier,
            bundleIdentifier: application.bundleIdentifier
        )
        guard PreviousApplicationTracking.shouldReactivate(remembered: previous, current: current)
        else { return false }
        return application.activate(options: [])
    }

    // MARK: - Setup

    private func resolveBundledLuma() -> URL? {
        guard let hostExecutableURL = Bundle.main.executableURL else {
            presentUnrecoverable("Could not determine the location of the Luma host executable.")
            return nil
        }
        let fileManager = FileManager.default
        let resolved = BundledExecutable.resolve(
            hostExecutableURL: hostExecutableURL,
            fileExists: { fileManager.fileExists(atPath: $0) },
            isExecutable: { fileManager.isExecutableFile(atPath: $0) }
        )
        switch resolved {
        case .success(let url):
            return url
        case .failure(let error):
            presentUnrecoverable(error.description)
            return nil
        }
    }

    private func registerHotKey(_ definition: HotKeyDefinition) {
        let controller = GlobalHotKeyController(definition: definition) { [weak self] in
            self?.handleHotKey()
        }
        do {
            try controller.register()
            hotKeyController = controller
            session?.activationShortcutDisplayName = definition.displayName
            UserDefaults.standard.set(
                definition.identifier,
                forKey: Self.activationShortcutDefaultsKey
            )
        } catch {
            let alternative = presentHotKeyRecovery(
                title: "\(HostIdentity.applicationName) could not register "
                    + "\(definition.displayName)",
                message: "\(error)\n\n"
                    + definition.registrationRecoveryMessage(
                        appPath: Bundle.main.bundleURL.path
                    ),
                alternatives: definition.alternatives()
            )
            if let alternative {
                registerHotKey(alternative)
            }
        }
    }

    // MARK: - Alerts

    private func presentHotKeyRecovery(
        title: String,
        message: String,
        alternatives: [HotKeyDefinition]
    ) -> HotKeyDefinition? {
        activateApplication()
        let alert = NSAlert()
        alert.alertStyle = .warning
        alert.messageText = title
        alert.informativeText = message
        for alternative in alternatives {
            alert.addButton(withTitle: "Use \(alternative.displayName)")
        }
        alert.addButton(withTitle: "Keep without shortcut")
        let response = alert.runModal()
        let index = response.rawValue - NSApplication.ModalResponse.alertFirstButtonReturn.rawValue
        guard index >= 0, index < alternatives.count else { return nil }
        return alternatives[index]
    }

    private func presentUnrecoverable(_ message: String) {
        activateApplication()
        let alert = NSAlert()
        alert.alertStyle = .critical
        alert.messageText = "\(HostIdentity.applicationName) cannot start"
        alert.informativeText = message
        alert.runModal()
        NSApp.terminate(nil)
    }
}

extension AppDelegate: LumaWindowControllerDelegate {
    func windowControllerDidRequestHide(_ controller: LumaWindowController) {
        apply(activation.hide())
    }
}

extension AppDelegate: TerminalSessionControllerDelegate {
    func terminalSessionDidStart(_ controller: TerminalSessionController) {
        memoryPressureController?.sampleNow()
    }

    /// Deliberately does not restart: the next activation starts a fresh session, so a binary that
    /// fails on startup cannot turn into a spawn loop.
    func terminalSessionDidTerminate(_ controller: TerminalSessionController) {}
}
