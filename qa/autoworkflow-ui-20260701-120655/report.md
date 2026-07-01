# Auto Workflow UI Acceptance Evidence

- Date: Wed Jul  1 12:06:55 HKT 2026
- Repo: /Users/diaoyuxuan/Luma
- App: /Users/diaoyuxuan/Luma/build/Luma.app
- State root: /Users/diaoyuxuan/.cc-loop

## Defaults

aw_path=/tmp/luma-aw-qa-source
aw_stateRoot=/Users/diaoyuxuan/.cc-loop
aw_planner=claude-code
aw_reviewer=claude-code
aw_implementer=cursor
aw_model=sonnet

(
    "luma.apps",
    "luma.autoworkflow",
    "luma.browser-tabs",
    "luma.clipboard",
    "luma.commands",
    "luma.kill-process",
    "luma.media",
    "luma.menu-items",
    "luma.notes",
    "luma.projects",
    "luma.quicklinks",
    "luma.secrets",
    "luma.snippets",
    "luma.todo",
    "luma.translate",
    "luma.window-layouts",
    "luma.wordbook"
)

## Processes

 2774 35368 S    Cursor Helper (Plugin): extension-host (user) Luma [10-23]                      
17940 35368 S    Cursor Helper (Plugin): extension-host (retrieval) Luma [10-30]                      
17941 35368 S    Cursor Helper (Plugin): extension-host (agent-exec) Luma [10-31]                      
34242     1 S    /Users/diaoyuxuan/Luma/build/Luma.app/Contents/MacOS/Luma
35617 61282 Ss   bash ./scripts/qa/autoworkflow_collect.sh

## cc-loop list

[
  {
    "task_id": "task-ui-recheck-1158",
    "status": "stopped",
    "target_repo": "/private/tmp/luma-aw-qa-source",
    "phase": "approved",
    "updated_at": "2026-07-01T04:06:41+00:00",
    "goal": "QA UI recheck: append a short acceptance note to README",
    "iteration": 1
  },
  {
    "task_id": "task-05EC41A0-73A",
    "status": "stopped",
    "target_repo": "/private/tmp/luma-aw-qa-source",
    "phase": "approved",
    "updated_at": "2026-07-01T03:43:44+00:00",
    "goal": "QA smoke goal: create a tiny harmless note for Luma Auto Workflow acceptance",
    "iteration": 1
  },
  {
    "task_id": "task-F113DD1B-DDD",
    "status": "stopped",
    "target_repo": "/private/tmp/luma-aw-qa-source",
    "phase": "planning",
    "updated_at": "2026-07-01T03:37:45+00:00",
    "goal": "QA smoke goal: create a tiny harmless note for Luma Auto Workflow acceptance",
    "iteration": 1
  },
  {
    "task_id": "task-5DC4B6C8-EFD",
    "status": "initialized",
    "target_repo": "/private/tmp/luma-aw-qa-source",
    "phase": "-",
    "updated_at": "2026-07-01T03:35:19+00:00",
    "goal": "QA smoke goal: create a tiny harmless note for Luma Auto Workflow acceptance",
    "iteration": 0
  }
]

## State files

/Users/diaoyuxuan/.cc-loop/tasks/task-ui-recheck-1158/state.json
/Users/diaoyuxuan/.cc-loop/tasks/task-ui-recheck-1158/runner.log
/Users/diaoyuxuan/.cc-loop/tasks/task-F113DD1B-DDD/state.json
/Users/diaoyuxuan/.cc-loop/tasks/task-F113DD1B-DDD/runner.log
/Users/diaoyuxuan/.cc-loop/tasks/task-05EC41A0-73A/state.json
/Users/diaoyuxuan/.cc-loop/tasks/task-05EC41A0-73A/runner.log
/Users/diaoyuxuan/.cc-loop/tasks/task-5DC4B6C8-EFD/runner.pid
/Users/diaoyuxuan/.cc-loop/tasks/task-5DC4B6C8-EFD/state.json
/Users/diaoyuxuan/.cc-loop/tasks/task-5DC4B6C8-EFD/runner.log

## Conclusion

Pass, with minor follow-ups.

The P1 fixes were verified through the real macOS UI, not only command-line tests. Auto Workflow detail layout now renders full-width in the launcher panel. A real workflow was started against `/tmp/luma-aw-qa-source`, showed task ID and PID, stopped cleanly, resumed via `cc-loop auto --detach` without the previous parse error, and was stopped again with no remaining `cc_loop`, `cc-loop`, or `claude --dangerously` runner process.

## Verified

- `swift test --filter AutoworkflowServiceTests` passed: 11/11.
- `./scripts/build_app.sh` passed and restarted `/Users/diaoyuxuan/Luma/build/Luma.app`.
- `./scripts/qa/autoworkflow_preflight.sh` confirmed `/tmp/luma-aw-qa-source` exists, is a git repo on `main`, Auto Workflow is enabled, and Luma GUI PATH can find `/opt/homebrew/anaconda3/bin/cc-loop`.
- `aw` detail layout is full-width; form fields, task list, and log tail are readable.
- Empty fields show `Goal and Repo are required`.
- Goal-only shows `Repo path is required`.
- Repo-only shows `Goal is required`.
- Settings source path availability changes immediately from Found to Not found while editing, and back to Found after restoring `/tmp/luma-aw-qa-source`.
- Start created `task-ui-recheck-1158`, reached real running state, and showed PID 25191.
- Stop removed PID 25191.
- Resume started detached runner again with PID 28143, then after the final status-display fix PID 34970, with no parse error.
- Final Stop removed PID 34970; final process check found no live `cc_loop`, `cc-loop`, or `claude --dangerously` process.

## Fix Applied During Recheck

- Adjusted `AutoworkflowDetailView` to treat `snapshot.isRunning` / `runner_pid` as the source of truth. This handles the transient cc-loop state where `status` can remain `running` while `running` is `false` and `next_action` is `resume`; the UI now shows Stopped + Resume instead of a misleading Running + Stop state.

## Remaining Follow-ups

- P2: The task list row can briefly show the `running` icon because `cc-loop list` reports `status:"running"` before status polling normalizes the selected task. The selected detail state is now correct.
- P2: Historical QA task `task-5DC4B6C8-EFD` still has a stale `runner.pid` file in the state root, but no matching process is alive. I left historical state files untouched.

## Screenshots

- `01-aw-detail-layout-full-width.png`
- `02-aw-validation-repo-required.png`
- `03-aw-validation-goal-required.png`
- `04-aw-running-with-task-pid.png`
- `05-aw-stopped-after-stop.png`
- `06-aw-resume-visible-after-stop.png`
- `07-aw-resumed-running-no-parse-error.png`
- `08-aw-stopped-after-final-stop.png`
- `09-settings-source-not-found-immediate.png`
- `10-settings-source-found-restored.png`
- `11-aw-stopped-resume-after-status-fix.png`
- `12-aw-resumed-after-status-fix.png`
- `13-aw-final-stopped-log-readable.png`
