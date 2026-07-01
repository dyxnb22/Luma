# Auto Workflow UI Acceptance

Use this checklist for the final real UI pass. The scripts collect environment
and evidence; the actual pass must still be done through the Luma UI.

## Recommended Runner

- Use GPT-5.5 with high reasoning for the main pass.
- Use GPT-5.4 with medium reasoning only for a second checklist review or evidence audit.
- The runner should operate the real macOS UI, not only run shell commands.

## Preflight

- Run `./scripts/build_app.sh`.
- Run `./scripts/qa/autoworkflow_preflight.sh`.
- If Auto Workflow is disabled, either enable it in Settings during the pass or run `./scripts/qa/autoworkflow_preflight.sh --enable-module`.
- Use the QA source path printed by the script, normally `/tmp/luma-aw-qa-source`, unless you intentionally want to test `~/autoworkflow`.
- Confirm the preflight finds `cc-loop` on the Luma GUI PATH, not only the shell PATH.

## Main UI Pass

- Open `build/Luma.app`.
- Press Command+Space and confirm the launcher appears with focus in the query field.
- Open Settings and enable Auto Workflow under Modules.
- Open Settings -> Auto Workflow and verify source path, state root, providers, model, source existence, and `cc-loop` availability.
- Press Command+Space again, type `aw`, and confirm the Auto Workflow detail panel opens in the same launcher surface.
- Try starting with an empty goal or missing repo and confirm the error is user-facing.
- Enter a small test goal and the QA source path, then start a real run.
- Confirm the UI shows task ID, PID, status, and a live log tail.
- Hide the launcher, reopen it, type `aw`, and confirm status resumes from `cc-loop status`.
- Stop the run and confirm the runner exits without killing unrelated processes.
- Resume the stopped/interrupted task when `cc-loop` exposes resume, and confirm status/logs update again.
- Test one negative path: invalid source path or temporarily unavailable `cc-loop`.

## Evidence

- Run `./scripts/qa/autoworkflow_collect.sh`.
- Save screenshots for Settings, `aw` empty/error state, running state, stopped state, and resumed state under the generated `qa/autoworkflow-ui-*` folder.
- Add a short conclusion to the generated report:
  - Pass / fail
  - Any blocking issue
  - Any follow-up polish item

## Acceptance Bar

- Command+Space works and does not regress normal launcher behavior.
- Auto Workflow stays default-off unless explicitly enabled.
- `aw` does not participate in hot-path global search work before targeted use.
- Real start, stop, and resume work against the external `cc-loop` CLI.
- Errors are readable and do not expose raw Python/AppKit noise as the primary message.
- No orphaned `cc-loop` or Luma child process remains after stop.
