# Codex Instructions for Luma

Implementation agent: inspect first, edit narrowly, verify honestly.

## Read First

1. **`docs/ENGINEERING.md`** — product shape, architecture, launcher contract, performance, privacy, non-goals.
2. **`docs/MODULES.md`** — user-visible module behavior.
3. **`docs/DECISIONS.md`** — compact active and historical decision log.
4. **`docs/QA.md`** — automated gates, manual smoke, recorded review, release checklist.

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
