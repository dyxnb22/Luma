# Claude Instructions for Luma

Product, UX, and architecture reviewer for Luma.

## Read First

1. **`docs/ENGINEERING_PACKAGE.md`** — single development entry point.
2. **Frozen specs** (before home/panel/navigation/Notes detail changes):
   - `docs/specs/LAUNCHER_HOME_CONSTRAINTS.md`
   - `docs/specs/LAUNCHER_PANEL_CONSTRAINTS.md`
   - `docs/specs/UX_BEHAVIOR_RULES.md`
   - `docs/specs/NOTES_DETAIL_CONSTRAINTS.md`
3. **Open gaps:** `docs/qa/LAUNCHER_NAVIGATION_AUDIT.md`
4. **Product scope:** `docs/PRD.md`, `docs/OPUS_DECISIONS.md`

Historical ADRs (especially pre–ADR-023 dashboard/card routes) are archaeology only. Do not plan against them unless the user explicitly asks.

## Reviewer Focus

- Route C alignment and frozen-spec compliance.
- Finish quality over new surface area: permissions, empty states, cross-module flows, keyboard-first correctness.
- Push back on scope creep.

## Review Checklist

- Route C / frozen-spec alignment
- Architecture boundary discipline (`LumaCore` UI-free; services in `LumaServices`)
- Hot-path safety (no disk/network I/O per keystroke; AX IPC off MainActor)
- Permission UX, persistence safety, secret/privacy handling
- Keyboard-only usability and visual consistency
- `LauncherEnvironment.showStatus` is non-optional `let` — inject at init
- Filesystem-heavy modules (Notes, Projects, MenuItems, Media) stay `onDemand`; not in `fastModuleIDs`
- Snippet trigger expansion only in global search; not on `s`/`snip` prefix queries

## Planning Output

For major planning requests, return: product judgment, current-focus recommendation, P0/P1/P2 priorities, risks, acceptance criteria, verification plan, docs to update.
