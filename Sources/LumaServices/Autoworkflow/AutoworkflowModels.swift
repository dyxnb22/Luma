import Foundation

// MARK: - Run status

public enum AutoworkflowRunStatus: Sendable, Codable, Equatable {
    case idle
    case initialized
    case running
    case stopped
    case done
    case failed
    case interrupted
    case waitingManualReview
    case unknown(String)

    public init(rawValue: String) {
        switch rawValue {
        case "idle": self = .idle
        case "initialized": self = .initialized
        case "running": self = .running
        case "stopped": self = .stopped
        case "done": self = .done
        case "failed": self = .failed
        case "interrupted": self = .interrupted
        case "waiting_manual_review": self = .waitingManualReview
        default: self = .unknown(rawValue)
        }
    }

    public var rawValue: String {
        switch self {
        case .idle: "idle"
        case .initialized: "initialized"
        case .running: "running"
        case .stopped: "stopped"
        case .done: "done"
        case .failed: "failed"
        case .interrupted: "interrupted"
        case .waitingManualReview: "waiting_manual_review"
        case .unknown(let value): value
        }
    }

    public var displayName: String {
        switch self {
        case .idle: "Idle"
        case .initialized: "Initialized"
        case .running: "Running"
        case .stopped: "Stopped"
        case .done: "Completed"
        case .failed: "Failed"
        case .interrupted: "Interrupted"
        case .waitingManualReview: "Review Needed"
        case .unknown(let value): value
        }
    }

    public var isTerminal: Bool {
        switch self {
        case .done, .failed, .stopped: true
        default: false
        }
    }

    public var isActive: Bool {
        switch self {
        case .initialized, .running, .waitingManualReview: true
        default: false
        }
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        self.init(rawValue: try container.decode(String.self))
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        try container.encode(rawValue)
    }
}

// MARK: - Attempt info

public struct AutoworkflowAttempt: Sendable, Codable {
    public let iteration: Int
    public let retry: Int
    public let phase: String
    public let decision: String
    public let testStatus: String
    public let implementerExitCode: Int
    public let worktreePath: String
    public let mergeError: String
    public let artifactDir: String
    public let createdAt: String

    public init(
        iteration: Int = 0,
        retry: Int = 0,
        phase: String = "",
        decision: String = "",
        testStatus: String = "",
        implementerExitCode: Int = 0,
        worktreePath: String = "",
        mergeError: String = "",
        artifactDir: String = "",
        createdAt: String = ""
    ) {
        self.iteration = iteration
        self.retry = retry
        self.phase = phase
        self.decision = decision
        self.testStatus = testStatus
        self.implementerExitCode = implementerExitCode
        self.worktreePath = worktreePath
        self.mergeError = mergeError
        self.artifactDir = artifactDir
        self.createdAt = createdAt
    }
}

// MARK: - Failure info

public struct AutoworkflowFailure: Sendable, Codable {
    public let failureType: String
    public let disposition: String
    public let stopReason: String
    public let recoveryRetryCount: Int
    public let mergeRetryCount: Int
    public let attemptedRepairs: [String]
    public let suggestedActions: [String]
    public let details: [String: String]

    public init(
        failureType: String = "",
        disposition: String = "",
        stopReason: String = "",
        recoveryRetryCount: Int = 0,
        mergeRetryCount: Int = 0,
        attemptedRepairs: [String] = [],
        suggestedActions: [String] = [],
        details: [String: String] = [:]
    ) {
        self.failureType = failureType
        self.disposition = disposition
        self.stopReason = stopReason
        self.recoveryRetryCount = recoveryRetryCount
        self.mergeRetryCount = mergeRetryCount
        self.attemptedRepairs = attemptedRepairs
        self.suggestedActions = suggestedActions
        self.details = details
    }
}

// MARK: - Task snapshot (from status --json)

public struct AutoworkflowTaskSnapshot: Sendable, Codable {
    public let taskID: String
    public let goal: String
    public let targetRepo: String
    public let baseBranch: String
    public let baseCommit: String
    public let status: AutoworkflowRunStatus
    public let iteration: Int
    public let attempt: AutoworkflowAttempt
    public let failure: AutoworkflowFailure
    public let nextAction: String
    public let isRunning: Bool
    public let runnerPID: Int?

    public init(
        taskID: String = "",
        goal: String = "",
        targetRepo: String = "",
        baseBranch: String = "",
        baseCommit: String = "",
        status: AutoworkflowRunStatus = .idle,
        iteration: Int = 0,
        attempt: AutoworkflowAttempt = .init(),
        failure: AutoworkflowFailure = .init(),
        nextAction: String = "",
        isRunning: Bool = false,
        runnerPID: Int? = nil
    ) {
        self.taskID = taskID
        self.goal = goal
        self.targetRepo = targetRepo
        self.baseBranch = baseBranch
        self.baseCommit = baseCommit
        self.status = status
        self.iteration = iteration
        self.attempt = attempt
        self.failure = failure
        self.nextAction = nextAction
        self.isRunning = isRunning
        self.runnerPID = runnerPID
    }
}

// MARK: - Task list item

public struct AutoworkflowTaskItem: Sendable, Codable {
    public let taskID: String
    public let status: String
    public let targetRepo: String
    public let phase: String
    public let updatedAt: String
    public let goal: String
    public let iteration: Int

    public init(
        taskID: String = "",
        status: String = "",
        targetRepo: String = "",
        phase: String = "",
        updatedAt: String = "",
        goal: String = "",
        iteration: Int = 0
    ) {
        self.taskID = taskID
        self.status = status
        self.targetRepo = targetRepo
        self.phase = phase
        self.updatedAt = updatedAt
        self.goal = goal
        self.iteration = iteration
    }

    public func withStatus(_ status: String) -> AutoworkflowTaskItem {
        AutoworkflowTaskItem(
            taskID: taskID,
            status: status,
            targetRepo: targetRepo,
            phase: phase,
            updatedAt: updatedAt,
            goal: goal,
            iteration: iteration
        )
    }
}

// MARK: - Configuration

public struct AutoworkflowConfig: Sendable, Codable {
    public var autoworkflowPath: String
    public var stateRoot: String
    public var defaultPlanner: String
    public var defaultReviewer: String
    public var defaultImplementer: String
    public var defaultModel: String

    public init(
        autoworkflowPath: String = "\(NSHomeDirectory())/autoworkflow",
        stateRoot: String = "\(NSHomeDirectory())/.cc-loop",
        defaultPlanner: String = "claude-code",
        defaultReviewer: String = "claude-code",
        defaultImplementer: String = "cursor",
        defaultModel: String = "sonnet"
    ) {
        self.autoworkflowPath = autoworkflowPath
        self.stateRoot = stateRoot
        self.defaultPlanner = defaultPlanner
        self.defaultReviewer = defaultReviewer
        self.defaultImplementer = defaultImplementer
        self.defaultModel = defaultModel
    }
}

// MARK: - CLI environment

public enum AutoworkflowCLIEnvironment {
    public static func environment(
        base: [String: String] = ProcessInfo.processInfo.environment,
        homeDirectory: String = NSHomeDirectory()
    ) -> [String: String] {
        var env = base
        env["PATH"] = path(base: base["PATH"], homeDirectory: homeDirectory)
        return env
    }

    public static func path(base: String?, homeDirectory: String = NSHomeDirectory()) -> String {
        var components = (base ?? "").split(separator: ":").map(String.init)
        let additions = [
            "/opt/homebrew/bin",
            "/opt/homebrew/anaconda3/bin",
            "/usr/local/bin",
            "/usr/local/anaconda3/bin",
            "\(homeDirectory)/.local/bin",
            "\(homeDirectory)/anaconda3/bin",
            "\(homeDirectory)/bin",
            "/usr/bin",
            "/bin",
            "/usr/sbin",
            "/sbin"
        ]
        for addition in additions where !components.contains(addition) {
            components.append(addition)
        }
        return components.joined(separator: ":")
    }
}

// MARK: - Structured error

public enum AutoworkflowError: Error, Sendable, LocalizedError {
    case notInstalled(path: String)
    case pathNotFound(path: String)
    case commandFailed(command: String, exitCode: Int, stderr: String)
    case invalidJSON(output: String)
    case noTaskFound
    case taskAlreadyRunning(taskID: String)
    case processTimeout
    case configInvalid(reason: String)
    case logUnavailable(path: String)

    public var errorDescription: String? {
        switch self {
        case .notInstalled(let path): "autoworkflow not installed at \(path)"
        case .pathNotFound(let path): "autoworkflow path not found: \(path)"
        case .commandFailed(let cmd, let code, let stderr): "Command failed: \(cmd) (exit \(code)): \(stderr)"
        case .invalidJSON(let output): "Failed to parse JSON from output: \(output.prefix(200))"
        case .noTaskFound: "No task found. Run init first."
        case .taskAlreadyRunning(let id): "Task \(id) is already running"
        case .processTimeout: "Process timed out"
        case .configInvalid(let reason): "Config invalid: \(reason)"
        case .logUnavailable(let path): "Log unavailable at \(path)"
        }
    }
}
