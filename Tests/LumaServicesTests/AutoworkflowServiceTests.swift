import Foundation
import LumaCore
import LumaServices
import Testing

@Suite struct AutoworkflowServiceTests {

    // MARK: - Test 1: Parse valid status JSON

    @Test func parseValidStatusJSON() throws {
        let json = """
        {
          "schema_version": 1,
          "cc_loop_version": "0.3.0",
          "task_id": "test-123",
          "goal": "Test goal",
          "target_repo": "/tmp/test-repo",
          "base_branch": "main",
          "base_commit": "abc123",
          "status": "running",
          "iteration": 2,
          "attempt": {
            "iteration": 2,
            "retry": 0,
            "phase": "test",
            "decision": "continue",
            "test_status": "passed",
            "implementer_exit_code": 0,
            "worktree_path": "/tmp/wt",
            "merge_error": "",
            "artifact_dir": "/tmp/artifacts",
            "created_at": "2026-07-01T00:00:00Z"
          },
          "failure": {
            "failure_type": "",
            "disposition": "",
            "stop_reason": "",
            "recovery_retry_count": 0,
            "merge_retry_count": 0,
            "attempted_repairs": [],
            "suggested_actions": [],
            "details": {}
          },
          "next_action": "none",
          "running": true,
          "runner_pid": 12345
        }
        """
        let snapshot = try AutoworkflowJSONCodec.decodeStatus(from: Data(json.utf8))

        #expect(snapshot.taskID == "test-123")
        #expect(snapshot.status == .running)
        #expect(snapshot.isRunning == true)
        #expect(snapshot.runnerPID == 12345)
    }

    // MARK: - Test 2: Parse initialized status

    @Test func parseInitializedStatus() throws {
        let json = """
        {
          "schema_version": 1,
          "task_id": "init-task",
          "goal": "",
          "target_repo": "/tmp/repo",
          "base_branch": "main",
          "base_commit": "",
          "status": "initialized",
          "iteration": 0,
          "attempt": {},
          "failure": {},
          "next_action": "start",
          "running": false
        }
        """
        let snapshot = try AutoworkflowJSONCodec.decodeStatus(from: Data(json.utf8))

        #expect(snapshot.status == .initialized)
        #expect(snapshot.isRunning == false)
        #expect(snapshot.runnerPID == nil)
    }

    // MARK: - Test 3: Parse unknown status

    @Test func parseUnknownStatus() throws {
        let json = """
        {
          "schema_version": 1,
          "task_id": "paused-task",
          "goal": "",
          "target_repo": "/tmp/repo",
          "base_branch": "main",
          "base_commit": "",
          "status": "paused",
          "iteration": 0,
          "next_action": "none",
          "running": false
        }
        """
        let snapshot = try AutoworkflowJSONCodec.decodeStatus(from: Data(json.utf8))

        #expect(snapshot.status == .unknown("paused"))
        #expect(snapshot.status != .idle)
        #expect(snapshot.status.isTerminal == false)
    }

    // MARK: - Test 4: Parse list JSON

    @Test func parseTaskListJSON() throws {
        let json = """
        [
          {
            "task_id": "task-alpha",
            "status": "running",
            "target_repo": "/tmp/repo-a",
            "phase": "implement",
            "updated_at": "2026-07-01T10:00:00Z",
            "goal": "First task",
            "iteration": 1
          },
          {
            "task_id": "task-beta",
            "status": "done",
            "target_repo": "/tmp/repo-b",
            "phase": "done",
            "updated_at": "2026-07-01T11:00:00Z",
            "goal": "Second task",
            "iteration": 3
          }
        ]
        """
        let tasks = try AutoworkflowJSONCodec.decodeTaskList(from: Data(json.utf8))

        #expect(tasks.count == 2)
        #expect(tasks[0].taskID == "task-alpha")
        #expect(tasks[0].status == "running")
    }

    // MARK: - Test 5: PID parsing from auto --detach output

    @Test func parseDetachedPIDFromOutput() {
        let output = "detached pid=4242 task_id=my-task log=/tmp/log"
        let pid = AutoworkflowJSONCodec.parseDetachedPID(from: output)

        #expect(pid == 4242)
    }

    // MARK: - Test 6: Extract JSON payload after prefixed logs

    @Test func extractPayloadSkipsPrefixedLogBrackets() throws {
        let output = """
        [info] checking task state
        {"task_id":"task-1","status":"running"}
        """
        let payload = try #require(AutoworkflowJSONCodec.extractPayload(from: output))

        #expect(payload.hasPrefix("{"))
        #expect(payload.contains(#""task_id":"task-1""#))
    }

    // MARK: - Test 7: Extract list payload

    @Test func extractPayloadHandlesArrayJSON() throws {
        let output = """
        cc-loop list
        [{"task_id":"task-alpha","status":"done"}]
        """
        let payload = try #require(AutoworkflowJSONCodec.extractPayload(from: output))

        #expect(payload.hasPrefix("["))
        #expect(payload.contains(#""task-alpha""#))
    }

    // MARK: - Test 8: Shell argument split helper

    @Test func shellSplitArguments() {
        #expect(autoworkflowShellSplitArguments("python -m pytest tests/ -q") == [
            "python", "-m", "pytest", "tests/", "-q"
        ])
        #expect(autoworkflowShellSplitArguments("echo 'hello world'") == [
            "echo", "hello world"
        ])
        #expect(autoworkflowShellSplitArguments("") == [])
        #expect(autoworkflowShellSplitArguments("simple") == ["simple"])
    }

    // MARK: - Test 9: CLI PATH includes common macOS install locations

    @Test func cliEnvironmentAugmentsPath() {
        let path = AutoworkflowCLIEnvironment.path(
            base: "/custom/bin:/usr/bin",
            homeDirectory: "/Users/tester"
        )
        let components = path.split(separator: ":").map(String.init)

        #expect(components.first == "/custom/bin")
        #expect(components.contains("/opt/homebrew/bin"))
        #expect(components.contains("/opt/homebrew/anaconda3/bin"))
        #expect(components.contains("/usr/local/anaconda3/bin"))
        #expect(components.contains("/Users/tester/.local/bin"))
        #expect(components.contains("/Users/tester/anaconda3/bin"))
        #expect(components.filter { $0 == "/usr/bin" }.count == 1)
    }

    // MARK: - Test 10: Status display names

    @Test func statusDisplayNames() {
        #expect(AutoworkflowRunStatus.done.displayName == "Completed")
        #expect(AutoworkflowRunStatus.running.displayName == "Running")
        #expect(AutoworkflowRunStatus.failed.displayName == "Failed")
    }
}
