import Foundation

// MARK: - Service protocol

public protocol AutoworkflowServiceProtocol: Sendable {
    /// Check if autoworkflow (cc-loop) is installed and available.
    func healthCheck() async -> Result<Bool, AutoworkflowError>

    /// Check if the autoworkflow source directory exists at configured path.
    func sourceExists(at path: String) async -> Bool

    /// Initialize a new task.
    func initializeTask(
        goal: String,
        repo: String,
        taskID: String?,
        testCommand: [String]?,
        config: AutoworkflowConfig
    ) async -> Result<String, AutoworkflowError>

    /// Start a task in background (auto --detach). Returns the PID.
    func startTask(taskID: String, config: AutoworkflowConfig) async -> Result<Int32, AutoworkflowError>

    /// Stop a running task by killing its runner process.
    func stopTask(taskID: String, config: AutoworkflowConfig) async -> Result<Void, AutoworkflowError>

    /// Resume a stopped or interrupted task via detached auto. Returns the runner PID.
    func resumeTask(taskID: String, config: AutoworkflowConfig) async -> Result<Int32, AutoworkflowError>

    /// Get task status snapshot.
    func taskStatus(taskID: String, config: AutoworkflowConfig) async -> Result<AutoworkflowTaskSnapshot, AutoworkflowError>

    /// List all tasks.
    func listTasks(config: AutoworkflowConfig) async -> Result<[AutoworkflowTaskItem], AutoworkflowError>

    /// Read runner log for a task.
    func readLog(taskID: String, config: AutoworkflowConfig, maxLines: Int) async -> Result<String, AutoworkflowError>

    /// Run doctor preflight checks.
    func runDoctor(repo: String, config: AutoworkflowConfig) async -> Result<Bool, AutoworkflowError>
}
