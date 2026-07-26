import Foundation

/// The application that was frontmost when Luma was summoned, so focus can be handed back.
public struct PreviousApplication: Equatable {
    public let processIdentifier: pid_t
    public let bundleIdentifier: String?

    public init(processIdentifier: pid_t, bundleIdentifier: String?) {
        self.processIdentifier = processIdentifier
        self.bundleIdentifier = bundleIdentifier
    }
}

public enum ActivationAction: Equatable {
    /// Bring the window forward. `startSession` is true when no child process is alive.
    case show(startSession: Bool)
    /// Order the window out and, when the target still exists, reactivate it.
    case hide(reactivate: PreviousApplication?)
}

/// Show/hide decisions for the single workbench window.
///
/// This is deliberately a value type with no AppKit references: the interesting part is the
/// policy (when to capture the previous app, when a hotkey means hide rather than show), and that
/// is exactly the part a GUI test cannot check reliably.
public struct ActivationPolicy: Equatable {
    public private(set) var isVisible: Bool
    public private(set) var previousApplication: PreviousApplication?

    public init(isVisible: Bool = false, previousApplication: PreviousApplication? = nil) {
        self.isVisible = isVisible
        self.previousApplication = previousApplication
    }

    /// Global hotkey pressed.
    ///
    /// Visible but not frontmost (buried behind another window) means "bring me forward", not
    /// "hide" — otherwise the hotkey would appear to do nothing.
    public mutating func toggle(
        applicationIsActive: Bool,
        frontmostApplication: PreviousApplication?,
        ownProcessIdentifier: pid_t,
        sessionIsRunning: Bool
    ) -> ActivationAction {
        if isVisible && applicationIsActive {
            return hide()
        }
        return show(
            frontmostApplication: frontmostApplication,
            ownProcessIdentifier: ownProcessIdentifier,
            sessionIsRunning: sessionIsRunning
        )
    }

    public mutating func show(
        frontmostApplication: PreviousApplication?,
        ownProcessIdentifier: pid_t,
        sessionIsRunning: Bool
    ) -> ActivationAction {
        if let frontmost = frontmostApplication, frontmost.processIdentifier != ownProcessIdentifier {
            previousApplication = frontmost
        }
        isVisible = true
        return .show(startSession: !sessionIsRunning)
    }

    /// Hiding consumes the remembered application: the next show captures a fresh one.
    @discardableResult
    public mutating func hide() -> ActivationAction {
        let target = previousApplication
        previousApplication = nil
        isVisible = false
        return .hide(reactivate: target)
    }
}

public enum PreviousApplicationTracking {
    /// PIDs are recycled. Before handing focus back, the application still running under the
    /// remembered PID must be the same one we left, or focus could land somewhere random.
    public static func shouldReactivate(
        remembered: PreviousApplication,
        current: PreviousApplication
    ) -> Bool {
        remembered.processIdentifier == current.processIdentifier
            && remembered.bundleIdentifier == current.bundleIdentifier
    }
}

/// Drops duplicate hotkey deliveries (key repeat, a stuck modifier, an impatient double press)
/// so a burst cannot race two show/hide transitions against each other.
public struct HotKeyDebouncer {
    public let minimumInterval: TimeInterval
    private var lastAccepted: TimeInterval?

    public init(minimumInterval: TimeInterval = 0.12) {
        self.minimumInterval = minimumInterval
    }

    public mutating func shouldAccept(at timestamp: TimeInterval) -> Bool {
        if let last = lastAccepted, timestamp - last < minimumInterval {
            return false
        }
        lastAccepted = timestamp
        return true
    }
}
