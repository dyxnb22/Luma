# Auto Workflow UI Acceptance Evidence

- Date: Wed Jul  1 11:57:53 HKT 2026
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
22539     1 S    /Users/diaoyuxuan/Luma/build/Luma.app/Contents/MacOS/Luma
23035 61282 Ss   bash ./scripts/qa/autoworkflow_collect.sh

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

