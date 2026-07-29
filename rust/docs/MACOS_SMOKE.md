# macOS smoke checks

These checks cover behavior that Rust unit tests and fake ports cannot prove: TCC permissions,
AppKit lifecycle, real window matching, terminal restoration, Keychain, paste synthesis, and
system proxy integration. They are intentionally module-local checks, not a centralized doctor.

Run the automated baseline first:

```bash
cd rust
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p luma --test cli_blackbox
./scripts/check_architecture.sh
```

## Before testing

- Use a disposable macOS user or a clearly labeled test data root where practical.
- Preserve the current system proxy configuration before touching `/proxy`.
- Do not use real secrets, private SSH hosts, or sensitive clipboard content.
- Test a TUI launched directly from Terminal and one hosted by `Luma.app` separately: macOS TCC
  permissions are per app/process.

## Permission and window checks

1. Revoke Accessibility from the app that launches Luma.
2. Open `/win` and confirm that the list/search surface remains available.
3. Attempt to focus a window and confirm the action reports `Permission required` with remediation.
4. Grant Accessibility and retry focus.
5. Open two windows with the same title, then refresh and focus each one from `/win`. The selected
   stable window must be raised; a refresh must not retarget a different row.
6. Repeat from Terminal and `Luma.app` with intentionally different permissions. Each process must
   show its own permission state and must not infer the other process's TCC state.

## Screen OCR checks

Run these checks once from Terminal and once from the installed `Luma.app`; Screen Recording
permission is process/bundle specific.

1. Revoke Screen Recording permission from the current launcher, run `/ocr`, and confirm the
   module reports `Permission required` with System Settings guidance without opening a selector.
2. Grant Screen Recording permission, restart that launcher if macOS requests it, run `/ocr`, and
   confirm the system region selector appears.
3. Press Esc in the region selector. Luma must report `Cancelled`, leave the clipboard unchanged,
   and remain usable.
4. Select a fixture containing English, Simplified Chinese, and Traditional Chinese. Confirm the
   recognized plain text is pasted into the active field and each script is recognizable.
5. Select a blank region and confirm `/ocr` reports an empty-result state instead of pasting.
6. Repeat a successful capture, a cancellation, and a forced recognition failure, then check the
   system temporary directory contains no leftover `.luma-ocr-*` capture.
7. Confirm neither the selected image path nor recognized text appears in Luma logs or Recall.
8. Use a dense fixture that would exceed 256 KiB and confirm pasted output is truncated on a UTF-8
   boundary. Cancel during selection and immediately after capture where practical; no later
   paste may occur.

## Local utility module checks

Use disposable package/database/renewal targets and non-sensitive command history.

1. In `/dl`, confirm only direct Downloads children appear. Replace a listed fixture before acting
   and verify stale identity is rejected. Rename a fixture, exercise the extension-change
   confirmation, move another fixture to Finder Trash, and restore it from Trash.
2. In `/pkg`, compare installed/outdated results with `brew`, then use a disposable formula or cask
   to verify confirmed install/upgrade/uninstall opens the exact Homebrew command in the
   interactive terminal. Do not test cleanup, taps, services, or the real daily toolchain.
3. In `/sc`, list folders, filter a folder containing Unicode names, View an exact shortcut, and
   run a harmless disposable shortcut. Duplicate exact names must be refused; no implicit input or
   captured output should be introduced.
4. In `/hist`, confirm safe plain and extended zsh-history rows can only be copied. Verify commands
   containing tokens, authorization headers, URL credentials, `curl -u user:pass`, and
   password-bearing database flags are hidden and never enter Recall or logs.
5. In `/renew`, use disposable rows to verify January 31 monthly advancement, leap-day yearly
   advancement, one-time completion, confirmed cancellation/deletion, stale-row rejection, and a
   reopenable metadata backup.
6. Enable `/db` explicitly. Add a disposable SQLite file, inspect tables/schema, open `sqlite3`,
   reveal it, and back up portal metadata. Confirm removing the portal leaves the database file.
7. Add a disposable PostgreSQL portal that relies on existing libpq authentication or an
   interactive `psql` prompt. Verify production open confirmation and confirm no password or DSN
   appears in rows, payloads, logs, Recall, or `database_portals.sqlite`.
8. Exercise Calculator i64 boundaries, unit/base/date examples, and a non-expression bare query;
   only a strict complete expression may contribute to global search.

## Workbench host checks (ADR-0007)

Build and install the host first, then work through the list. These are the checks that cannot be
automated: they need real activation, a real input method, and a real GPU-composited window.

```bash
cd rust
bash scripts/build_workbench_app.sh "$HOME/Applications/Luma.app"
/usr/bin/codesign --verify --deep --strict "$HOME/Applications/Luma.app"
open "$HOME/Applications/Luma.app"
```

The default ad-hoc signature is suitable for this local smoke build, not a promise that TCC
permissions will persist across rebuilds. Re-check Accessibility (and any other module-local
permission) after replacing the bundle; use `CODESIGN_IDENTITY` with a stable local certificate
when continuity matters.

1. **Cold launch** — the window appears centered, the TUI Hub renders, and no Dock icon is added.
2. **Warm activation** — click another app, press the saved activation shortcut (fresh default
   Option+Space); Luma comes forward on the current
   Space with the terminal already focused and the previous TUI state intact. Repeat while the
   other app is full-screen; Luma must join that Space instead of remaining behind it.
3. **Hide and restore focus** — press the saved activation shortcut again; the window hides and the app you came
   from becomes frontmost.
4. **Rapid toggling and minimize** — hold/repeat the saved activation shortcut quickly; exactly one window and one
   `luma` process must exist afterwards (`pgrep -fl 'Luma.app/Contents/MacOS/luma'`). Minimize the
   window and press the saved shortcut; the same window must deminiaturize on the first press.
5. **Chinese IME** — switch to Pinyin, type a multi-syllable word, confirm the composition marks
   correctly and only the committed text reaches the prompt.
6. **CJK alignment** — display Records rows with mixed CJK/ASCII and confirm columns line
   up (full-width cells occupy two columns).
7. **Keyboard paging** — populate enough Hub, Results, Help, Preview, Settings, command-palette,
   and ActionPicker rows to overflow. Verify `⌥↑` / `⌥↓`, the compact-Mac `fn↑` / `fn↓` alias,
   and the `Ctrl-/` palette actions
   `/scroll up` / `/scroll down` clamp at both ends, keep the current surface/focus, and never run
   the highlighted action or trigger a refresh.
8. **Mouse** — plain terminal click/drag must select text normally; the keyboard-first TUI does
   not claim mouse reporting and must not block native selection.
9. **Copy/paste** — Cmd+C with an active selection, then Cmd+V into the prompt. Paste a
   multi-line value while an action confirmation is visible: it must not confirm or run anything.
10. **Resize and visual focus** — resize the window and confirm the TUI reflows without artifacts.
    Verify exactly one pane has the accent focus border, the selected row fills its available
    width, overlays retain an opaque raised panel over the dimmed workbench, and the contextual
    shortcut bar remains on the last row. Enter and leave a full-screen child surface to exercise
    the alternate screen.
11. **Timer while hidden** — start a Timer, hide the window, wait past the deadline, and re-show;
    the timer must have kept running (the child process is not suspended).
12. **`/wb review`** — run today's due-first review queue, reveal and grade a card, then Esc out; Escape must reach
    the TUI and must never close or hide the window.
13. **Interactive child** — run `/ssh` against a host alias that does not resolve (no real remote
    connection needed) and confirm the child runs in the same PTY and the TUI resumes afterwards.
14. **Command Recipe** — run a recipe with `/cmd test` and confirm output and exit handling.
15. **TUI exit and restart** — Ctrl-C out of the TUI, hide, then press the saved activation
    shortcut; a fresh
    session must start instead of an empty window.
16. **Quit and cleanup** — start a Timer, then Cmd+Q. Confirm no `luma` child survives
    (`pgrep -fl luma`) and the Timer was persisted as paused, proving graceful module teardown ran.
17. **Relaunch from the installed bundle** — reopen `$HOME/Applications/Luma.app` and confirm the
    window frame was restored and the hotkey works again.
18. **Login item (optional, manual)** — the host ships no Launch at Login toggle. If you add
    `Luma.app` under System Settings → General → Login Items, confirm after a reboot that exactly
    one host and one child process are running.
19. **Memory policy** — inspect the local unified log after normal use and confirm the exit entry
    reports combined peak RSS. Triggering real warning/critical pressure is optional; when tested,
    confirm the entry reports scrollback reduction and the TUI remains usable.

If another launcher owns the saved shortcut, the host must report registration failure and offer
explicit alternatives instead of silently changing it. Command+Space users may also disable or
rebind Spotlight under System Settings → Keyboard → Keyboard Shortcuts.

## Terminal suspend/resume checks

Run a short SSH or command-recipe action in the TUI and verify each path:

- child exits successfully;
- child exits non-zero;
- child is interrupted with Ctrl-C;
- child fails to start;
- TUI suspend or resume reports an error.

After every path, the shell must accept input, the cursor must be visible, raw mode must be off
outside the TUI, and the alternate screen must be restored when the TUI resumes. A failed suspend
must not be followed by an unconditional resume attempt.

## Clipboard, Keychain, and Proxy checks

- Without Accessibility, Clipboard history/search and Snippet search/copy remain usable; paste
  reports a local permission state.
- With a disposable Keychain service, set, unlock, copy, cancel, idle-lock, and lock again. Values
  must not appear in search rows or logs.
- For `/proxy`, save the current network service configuration, apply a safe test change, simulate
  an external change, and verify divergence forces a safe restore or a clear unavailable state.
  Restore the original configuration afterward.

## Frequency

- Every commit: automated baseline and pure model/reducer tests.
- After touching TerminalGuard, the workbench host, or platform adapters: run the relevant section
  above.
- Before relying on a new macOS build: run the complete checklist manually.
