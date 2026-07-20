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
- Test the TUI and the menu-bar companion separately: macOS TCC permissions are per app/process.

## Permission and window checks

1. Revoke Accessibility from the app that launches Luma.
2. Open `/win` and confirm that the list/search surface remains available.
3. Attempt to focus a window and confirm the action reports `Permission required` with remediation.
4. Grant Accessibility and retry focus.
5. Open two windows with the same title, then refresh and focus each one from `/win` and the menu
   bar. The selected stable window must be raised; a refresh must not retarget a different row.
6. Repeat with TUI and menu-bar permissions intentionally different. Each surface must show its
   own permission state and must not infer the other process's TCC state.

## Menu-bar lifecycle checks

- Start the companion twice; the second instance must exit cleanly with a duplicate-instance
  message.
- Open the menu, refresh it, close and reopen it, and verify Wordbook/Windows rows update.
- Trigger refresh while windows are being opened or closed. A stale snapshot may be labeled stale,
  but the companion must never focus a different displayed row silently.
- Remove or override the CLI path and verify Open Luma, Open Settings, and Review Due show a clear
  local failure instead of doing nothing.
- Exercise Launch at Login in an unbundled build and a bundled app. Verify Enabled,
  Requires Approval, Not Registered, and Not Found are not collapsed into one boolean.
- Quit from the menu, wait for termination, then relaunch. The instance lock must be released.

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
- After touching TerminalGuard, menu-bar worker/model, or platform adapters: run the relevant
  section above.
- Before relying on a new macOS build: run the complete checklist manually.

