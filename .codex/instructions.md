# Codex Instructions for Luma

Implementation agent: inspect first, edit narrowly, verify honestly.

## Read First

1. **`docs/ENGINEERING_PACKAGE.md`** — single development entry point.
2. **Frozen specs:**
   - `docs/specs/LAUNCHER_HOME_CONSTRAINTS.md`
   - `docs/specs/LAUNCHER_PANEL_CONSTRAINTS.md`
   - `docs/specs/UX_BEHAVIOR_RULES.md`
   - `docs/specs/NOTES_DETAIL_CONSTRAINTS.md`
3. **Open gaps:** `docs/qa/LAUNCHER_NAVIGATION_AUDIT.md`

Do not implement dashboard/card or Route B home instructions from historical ADRs unless explicitly requested.

## Implementation Bias

- Integration fixes over new modules.
- Keep command modules prefix-triggered; `handle` memory-only on the query path.
- `LumaCore` UI-free; system boundaries in `LumaServices`; launcher UI in `LumaApp`.
- `LauncherEnvironment.showStatus` and other callbacks: `let` parameters in `init`, never optional `var`.
- Filesystem-heavy modules (Notes, Projects, MenuItems, Media): `onDemand`, not in `fastModuleIDs`.
- AX IPC off MainActor; only `frontmostApplication` capture on main.

## Verification

- `swift build` — normal Swift edits
- `swift test` — module, ranking, persistence, action changes
- `./scripts/build_app.sh` — app bundle / launcher runtime
- `./scripts/qa/run_full_smoke.sh` — material launcher UX changes

Report exactly what ran and what did not.
