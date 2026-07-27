import Foundation

public enum ChildProcessGroup {
    /// `forkpty` makes the child a process-group leader. A negative PID targets the whole group,
    /// including an interactive SSH or recipe child, while rejecting unsafe/non-child values.
    public static func signalTarget(forLeader processIdentifier: pid_t) -> pid_t? {
        processIdentifier > 1 ? -processIdentifier : nil
    }
}

/// State of the single `luma tui` child process.
public enum SessionTermination: Equatable {
    case exited(code: Int32)
    case signaled(signal: Int32)
    case unknown

    /// SwiftTerm 1.15's macOS forkpty path forwards the raw `waitpid` status even though the
    /// delegate parameter is named `exitCode`. Decode that pinned behavior here so host feedback
    /// does not describe a signal number as a normal exit code.
    public static func fromSwiftTermWaitStatus(_ status: Int32?) -> Self {
        guard let status else { return .unknown }
        let signal = status & 0x7f
        if signal == 0 {
            return .exited(code: (status >> 8) & 0xff)
        }
        if signal != 0x7f {
            return .signaled(signal: signal)
        }
        return .unknown
    }

    fileprivate var noticeDetail: String {
        switch self {
        case .exited(let code): "exit code \(code)"
        case .signaled(let signal): "signal \(signal)"
        case .unknown: "unknown exit status"
        }
    }
}

public enum SessionState: Equatable {
    case notStarted
    case running
    case exited(SessionTermination)
}

/// Tracks the one child process the host is allowed to own.
///
/// The host never restarts the child on its own: an exited session is remembered and a fresh one
/// starts on the next activation. That keeps a binary that crashes on startup from turning into a
/// spawn loop.
public struct SessionLifecycle: Equatable {
    public private(set) var state: SessionState

    public init(state: SessionState = .notStarted) {
        self.state = state
    }

    public var isRunning: Bool { state == .running }

    /// Whether showing the window should start a session first.
    public var needsStartBeforeShowing: Bool { state != .running }

    public mutating func markStarted() {
        state = .running
    }

    public mutating func markTerminated(swiftTermWaitStatus: Int32?) {
        state = .exited(.fromSwiftTermWaitStatus(swiftTermWaitStatus))
    }

    /// Short, honest text for the terminal after the child goes away. Not an error page.
    public var terminationNotice: String? {
        guard case .exited(let termination) = state else { return nil }
        return "[luma tui ended — \(termination.noticeDetail). Press ⌘Space to start a new session.]"
    }
}
