# Auto Workflow UI Acceptance Evidence

- Date: Wed Jul  1 11:46:10 HKT 2026
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
11512     1 S    /Users/diaoyuxuan/Luma/build/Luma.app/Contents/MacOS/Luma
14464 61282 Ss   bash ./scripts/qa/autoworkflow_collect.sh

## cc-loop list

[
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

/Users/diaoyuxuan/.cc-loop/tasks/task-F113DD1B-DDD/state.json
/Users/diaoyuxuan/.cc-loop/tasks/task-F113DD1B-DDD/runner.log
/Users/diaoyuxuan/.cc-loop/tasks/task-05EC41A0-73A/state.json
/Users/diaoyuxuan/.cc-loop/tasks/task-05EC41A0-73A/runner.log
/Users/diaoyuxuan/.cc-loop/tasks/task-5DC4B6C8-EFD/runner.pid
/Users/diaoyuxuan/.cc-loop/tasks/task-5DC4B6C8-EFD/state.json
/Users/diaoyuxuan/.cc-loop/tasks/task-5DC4B6C8-EFD/runner.log

## Conclusion

Fail.

Luma is launchable with Command+Space, Auto Workflow can be enabled through Settings, `aw` opens the same launcher detail surface, real `cc-loop` start reaches live external providers, hidden/reopened state restores from `cc-loop status`, user-facing error states are readable, and Stop cleanup passed after a Luma fix. The final acceptance does not pass because Resume does not work against the current `cc-loop resume` command surface, and the Auto Workflow detail view is visually compressed into narrow columns that make the UI hard to use.

## Blocking Issues

- P1: Resume fails. Luma expects resume to return detached `pid=...` output, but current `cc-loop resume` returns human/status text. UI shows a readable error, but resume does not continue the task.
- P1: Auto Workflow detail layout is severely compressed in the launcher panel. The form, task list, and log tail render as narrow vertical columns; accessibility text is present, but the visible UI is not acceptable for final release.

## Fixed During Pass

- Fixed Luma polling/stop state handling in `Sources/LumaApp/Launcher/AutoworkflowDetailView.swift`: `snapshot.isRunning` / `runner_pid` now keeps Stop available even when `status` is already `stopped`, and the status detail includes task ID/PID.
- Fixed local `cc-loop` detached runner in `/Users/diaoyuxuan/autoworkflow/src/cc_loop/detach.py`: child argv now places global `--state-root` before the `auto` subcommand.

## Polish Follow-ups

- Settings -> Auto Workflow availability does not immediately update when the source path text field changes before/after Save.
- Empty goal vs missing repo uses the same message, `Goal and Repo are required`; readable, but not specific.
- Log tail can contain repeated blocks after polling/resume attempts; consider deduping or bounded append behavior in the UI.
- Preflight creates `/tmp/luma-aw-qa-source` but does not initialize it as a git repo on `main`, while real `cc-loop doctor` requires both.

## Exact Commands And Scripts Run

- `./scripts/build_app.sh`
- `./scripts/qa/autoworkflow_preflight.sh`
- `./scripts/qa/autoworkflow_collect.sh`
- `git -C /tmp/luma-aw-qa-source init`
- `git -C /tmp/luma-aw-qa-source add README.md`
- `git -C /tmp/luma-aw-qa-source -c user.name='Luma QA' -c user.email='luma-qa@example.invalid' commit -m 'Initialize Luma Auto Workflow QA source'`
- `git -C /tmp/luma-aw-qa-source branch -m main`
- `cc-loop --state-root /Users/diaoyuxuan/.cc-loop status --task-id task-5DC4B6C8-EFD --json`
- `cc-loop --state-root /Users/diaoyuxuan/.cc-loop auto --detach --task-id task-5DC4B6C8-EFD`
- `/opt/homebrew/anaconda3/bin/python -m pytest /Users/diaoyuxuan/autoworkflow/tests/test_cli_contract.py -q`
- `./scripts/build_app.sh`
- `cc-loop --state-root /Users/diaoyuxuan/.cc-loop list --json`
- `cc-loop --state-root /Users/diaoyuxuan/.cc-loop status --task-id task-F113DD1B-DDD --json`
- `cc-loop --state-root /Users/diaoyuxuan/.cc-loop status --task-id task-05EC41A0-73A --json`
- `swift test --filter AutoworkflowServiceTests`

## Process Cleanup

- UI Stop initially left runner PID 7169 alive for `task-05EC41A0-73A`; after the Luma fix and rebuild, selecting the task exposed Stop again and UI Stop removed PID 7169 plus its `runner.pid`.
- Final process check found no remaining `cc_loop`, `cc-loop`, or `claude --dangerously` runner processes. Existing unrelated Cursor agent processes were left untouched.
