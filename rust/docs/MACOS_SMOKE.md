# macOS smoke checks

These checks cover behavior that Rust unit tests and fake ports cannot prove: TCC permissions,
AppKit lifecycle, real window matching, terminal restoration, and paste synthesis. They are
module-local checks, not a centralized doctor.

Run the automated baseline first:

```bash
cd rust
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p luma --test cli_blackbox
./scripts/check_architecture.sh
```

## Permission, Windows, and Clipboard

1. Revoke Accessibility from the app that launches Luma. `/win` must still list/search windows,
   while Focus reports `Permission required` with local remediation.
2. Grant Accessibility and retry Focus. Test two windows with the same title and verify refreshes
   do not retarget the selected stable identity.
3. Repeat from Terminal and `Luma.app` with intentionally different permissions; each launcher
   must report its own TCC state.
4. Exercise Clipboard history, pin/unpin, pause/resume, clear, and paste. Concealed/transient
   password-manager pasteboard types must not be captured. Without Accessibility, paste reports a
   local permission state while history/search remains usable.
5. Do not use sensitive clipboard content. Tests and diagnostics must not mutate the user's real
   clipboard or steal focus.

## Workbench host (ADR-0007)

Build and install the host first:

```bash
cd rust
bash scripts/build_workbench_app.sh "$HOME/Applications/Luma.app"
/usr/bin/codesign --verify --deep --strict "$HOME/Applications/Luma.app"
open "$HOME/Applications/Luma.app"
```

The default ad-hoc signature is convenient for local builds but does not guarantee TCC continuity
after replacement. Use a stable local `CODESIGN_IDENTITY` when that matters.

1. **Cold launch** — one centered TUI window appears and no Dock icon is added.
2. **Activation** — the saved shortcut (fresh default Option+Space) brings the same session to the
   current Space; pressing it again hides Luma and restores the previous app.
3. **Rapid toggling** — repeated activation and deminiaturization still leave exactly one window
   and one `luma` child.
4. **IME and layout** — Chinese composition commits once; mixed CJK/ASCII Records rows align.
5. **Paging and mouse** — `⌥↑`/`⌥↓`, `fn↑`/`fn↓`, and `/scroll up|down` clamp without executing
   actions; ordinary terminal click/drag still selects text.
6. **Paste safety** — Cmd+C/Cmd+V works, and multiline paste into a confirmation never confirms or
   executes an action.
7. **Resize and focus** — panels reflow, exactly one pane has the focused accent, overlays remain
   opaque, and the shortcut footer remains visible.
8. **Wordbook** — reveal and grade a `/wb review` card, then Esc back to the TUI without hiding the
   host window.
9. **Interactive child** — run successful, failing, interrupted, and missing Command Recipes.
   The TUI must restore raw mode, cursor, alternate screen, and input after every outcome.
10. **Exit/restart** — exit the TUI, then activate again; a fresh child starts in the same host.
11. **Quit** — Cmd+Q terminates and reaps the child; no `luma` process survives.
12. **Memory policy** — local unified logs report combined peak RSS; optional pressure testing may
   reduce scrollback without making the TUI unusable.

If another launcher owns the hotkey, the host must offer explicit alternatives rather than
silently changing it. Launch at Login remains a manual System Settings choice.

## Frequency

- Every change: run the automated baseline.
- After touching the terminal guard, workbench host, or platform adapters: run the relevant manual
  section.
- Before relying on a rebuilt macOS app: run the complete checklist once.
