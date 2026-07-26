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
2. **Warm activation** — click another app, press Option+Space; Luma comes forward on the current
   Space with the terminal already focused and the previous TUI state intact. Repeat while the
   other app is full-screen; Luma must join that Space instead of remaining behind it.
3. **Hide and restore focus** — press Option+Space again; the window hides and the app you came
   from becomes frontmost.
4. **Rapid toggling and minimize** — hold/repeat Option+Space quickly; exactly one window and one
   `luma` process must exist afterwards (`pgrep -fl 'Luma.app/Contents/MacOS/luma'`). Minimize the
   window and press Option+Space; the same window must deminiaturize on the first press.
5. **Chinese IME** — switch to Pinyin, type a multi-syllable word, confirm the composition marks
   correctly and only the committed text reaches the prompt.
6. **CJK alignment** — display Records rows with mixed CJK/ASCII and confirm columns line
   up (full-width cells occupy two columns).
7. **Mouse** — plain terminal click/drag must select text normally; the keyboard-first TUI does
   not claim mouse reporting and must not block native selection.
8. **Copy/paste** — Cmd+C with an active selection, then Cmd+V into the prompt. Paste a
   multi-line value while an action confirmation is visible: it must not confirm or run anything.
9. **Resize** — resize the window and confirm the TUI reflows without artifacts; enter and leave a
   full-screen child surface to exercise the alternate screen.
10. **Timer while hidden** — start a Timer, hide the window, wait past the deadline, and re-show;
    the timer must have kept running (the child process is not suspended).
11. **`/wb review`** — run today's due-first review queue, reveal and grade a card, then Esc out; Escape must reach
    the TUI and must never close or hide the window.
12. **Interactive child** — run `/ssh` against a host alias that does not resolve (no real remote
    connection needed) and confirm the child runs in the same PTY and the TUI resumes afterwards.
13. **Command Recipe** — run a recipe with `/cmd test` and confirm output and exit handling.
14. **TUI exit and restart** — Ctrl-C out of the TUI, hide, then press Option+Space; a fresh
    session must start instead of an empty window.
15. **Quit and cleanup** — start a Timer, then Cmd+Q. Confirm no `luma` child survives
    (`pgrep -fl luma`) and the Timer was persisted as paused, proving graceful module teardown ran.
16. **Relaunch from the installed bundle** — reopen `$HOME/Applications/Luma.app` and confirm the
    window frame was restored and the hotkey works again.
17. **Login item (optional, manual)** — the host ships no Launch at Login toggle. If you add
    `Luma.app` under System Settings → General → Login Items, confirm after a reboot that exactly
    one host and one child process are running.
18. **Memory policy** — inspect the local unified log after normal use and confirm the exit entry
    reports combined peak RSS. Triggering real warning/critical pressure is optional; when tested,
    confirm the entry reports scrollback reduction and the TUI remains usable.

If Option+Space is already claimed (Spotlight, an input-source switcher, another launcher), the
host reports the registration failure at startup instead of silently doing nothing.

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
