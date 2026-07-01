import Foundation

// MARK: - JSON decoding (cc-loop integration schema v1)

public enum AutoworkflowJSONCodec {
    public static func decodeStatus(from data: Data) throws -> AutoworkflowTaskSnapshot {
        let raw = try JSONDecoder().decode(RawStatusJSON.self, from: data)
        return mapStatus(raw)
    }

    public static func decodeTaskList(from data: Data) throws -> [AutoworkflowTaskItem] {
        try JSONDecoder().decode([RawTaskItem].self, from: data).map(mapTaskItem)
    }

    public static func parseDetachedPID(from output: String) -> Int32? {
        guard let pidMatch = output.range(of: #"pid=(\d+)"#, options: .regularExpression) else {
            return nil
        }
        return Int32(output[pidMatch].dropFirst(4))
    }

    public static func extractPayload(from output: String) -> String? {
        let trimmed = output.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return nil }

        for candidate in trimmed.indices where trimmed[candidate] == "{" || trimmed[candidate] == "[" {
            let payload = String(trimmed[candidate...])
            if let data = payload.data(using: .utf8),
               (try? JSONSerialization.jsonObject(with: data)) != nil {
                return payload
            }
        }
        return nil
    }
}

public func autoworkflowShellSplitArguments(_ input: String) -> [String] {
    var result: [String] = []
    var current = ""
    var inSingleQuote = false
    var inDoubleQuote = false
    var index = input.startIndex

    while index < input.endIndex {
        let char = input[index]
        if char == "'" && !inDoubleQuote {
            inSingleQuote.toggle()
            index = input.index(after: index)
        } else if char == "\"" && !inSingleQuote {
            inDoubleQuote.toggle()
            index = input.index(after: index)
        } else if char.isWhitespace && !inSingleQuote && !inDoubleQuote {
            if !current.isEmpty {
                result.append(current)
                current = ""
            }
            index = input.index(after: index)
        } else {
            current.append(char)
            index = input.index(after: index)
        }
    }
    if !current.isEmpty {
        result.append(current)
    }
    return result
}

// MARK: - Mapping

private func mapStatus(_ raw: RawStatusJSON) -> AutoworkflowTaskSnapshot {
    AutoworkflowTaskSnapshot(
        taskID: raw.task_id,
        goal: raw.goal,
        targetRepo: raw.target_repo,
        baseBranch: raw.base_branch,
        baseCommit: raw.base_commit,
        status: AutoworkflowRunStatus(rawValue: raw.status),
        iteration: raw.iteration,
        attempt: AutoworkflowAttempt(
            iteration: raw.attempt?.iteration ?? 0,
            retry: raw.attempt?.retry ?? 0,
            phase: raw.attempt?.phase ?? "",
            decision: raw.attempt?.decision ?? "",
            testStatus: raw.attempt?.test_status ?? "",
            implementerExitCode: raw.attempt?.implementer_exit_code ?? 0,
            worktreePath: raw.attempt?.worktree_path ?? "",
            mergeError: raw.attempt?.merge_error ?? "",
            artifactDir: raw.attempt?.artifact_dir ?? "",
            createdAt: raw.attempt?.created_at ?? ""
        ),
        failure: AutoworkflowFailure(
            failureType: raw.failure?.failure_type ?? "",
            disposition: raw.failure?.disposition ?? "",
            stopReason: raw.failure?.stop_reason ?? "",
            recoveryRetryCount: raw.failure?.recovery_retry_count ?? 0,
            mergeRetryCount: raw.failure?.merge_retry_count ?? 0,
            attemptedRepairs: raw.failure?.attempted_repairs ?? [],
            suggestedActions: raw.failure?.suggested_actions ?? [],
            details: raw.failure?.details ?? [:]
        ),
        nextAction: raw.next_action,
        isRunning: raw.running,
        runnerPID: raw.runner_pid.map { Int($0) }
    )
}

private func mapTaskItem(_ raw: RawTaskItem) -> AutoworkflowTaskItem {
    AutoworkflowTaskItem(
        taskID: raw.task_id,
        status: raw.status,
        targetRepo: raw.target_repo,
        phase: raw.phase,
        updatedAt: raw.updated_at,
        goal: raw.goal,
        iteration: raw.iteration
    )
}

// MARK: - Raw JSON decodable structs

private struct RawStatusJSON: Decodable {
    let schema_version: Int
    let cc_loop_version: String?
    let task_id: String
    let goal: String
    let target_repo: String
    let base_branch: String
    let base_commit: String
    let status: String
    let iteration: Int
    let attempt: RawAttempt?
    let failure: RawFailure?
    let next_action: String
    let running: Bool
    let runner_pid: Int32?
}

private struct RawAttempt: Decodable {
    let iteration: Int
    let retry: Int
    let phase: String
    let decision: String
    let test_status: String
    let implementer_exit_code: Int
    let worktree_path: String
    let merge_error: String
    let artifact_dir: String
    let created_at: String

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        iteration = try container.decodeIfPresent(Int.self, forKey: .iteration) ?? 0
        retry = try container.decodeIfPresent(Int.self, forKey: .retry) ?? 0
        phase = try container.decodeIfPresent(String.self, forKey: .phase) ?? ""
        decision = try container.decodeIfPresent(String.self, forKey: .decision) ?? ""
        test_status = try container.decodeIfPresent(String.self, forKey: .test_status) ?? ""
        implementer_exit_code = try container.decodeIfPresent(Int.self, forKey: .implementer_exit_code) ?? 0
        worktree_path = try container.decodeIfPresent(String.self, forKey: .worktree_path) ?? ""
        merge_error = try container.decodeIfPresent(String.self, forKey: .merge_error) ?? ""
        artifact_dir = try container.decodeIfPresent(String.self, forKey: .artifact_dir) ?? ""
        created_at = try container.decodeIfPresent(String.self, forKey: .created_at) ?? ""
    }

    private enum CodingKeys: String, CodingKey {
        case iteration, retry, phase, decision, test_status, implementer_exit_code
        case worktree_path, merge_error, artifact_dir, created_at
    }
}

private struct RawFailure: Decodable {
    let failure_type: String?
    let disposition: String?
    let stop_reason: String?
    let recovery_retry_count: Int?
    let merge_retry_count: Int?
    let attempted_repairs: [String]?
    let suggested_actions: [String]?
    let details: [String: String]?
}

private struct RawTaskItem: Decodable {
    let task_id: String
    let status: String
    let target_repo: String
    let phase: String
    let updated_at: String
    let goal: String
    let iteration: Int
}
