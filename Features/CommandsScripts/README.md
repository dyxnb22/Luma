# Scripted Commands

User-defined shell commands loaded from:

`~/Library/Application Support/Luma/commands.json`

## Schema

```json
{
  "commands": [
    {
      "id": "build",
      "title": "Build",
      "trigger": "build",
      "exec": "/bin/zsh",
      "args": ["-lc", "./scripts/build_app.sh"],
      "cwd": "{{project}}",
      "timeoutSec": 600,
      "showOutput": "notification"
    }
  ]
}
```

## Template variables

- `{{project}}` — matched project name from frontmost IDE
- `{{project_path}}` — matched project path
- `{{clipboard}}` — pasteboard text
- `{{selection}}` — selected text in frontmost app

## Examples

**Git pull in current project**

```json
{
  "id": "git-pull",
  "title": "Git Pull",
  "trigger": "pull",
  "exec": "/bin/zsh",
  "args": ["-lc", "git pull --ff-only"],
  "cwd": "{{project_path}}",
  "timeoutSec": 120
}
```

**Run tests**

```json
{
  "id": "test",
  "title": "Run Tests",
  "trigger": "test",
  "exec": "/bin/zsh",
  "args": ["-lc", "swift test"],
  "cwd": "{{project_path}}",
  "timeoutSec": 600
}
```

## Safety

- Scripts run only from `perform()` (never on the keystroke hot path).
- Timeout is capped at 600 seconds.
- Use **Reveal Config** on any command row to inspect `commands.json`.
