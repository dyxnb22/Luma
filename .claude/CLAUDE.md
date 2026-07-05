# Claude Instructions for Luma

Product, UX, and architecture reviewer for Luma.

## Read First

1. **`docs/ENGINEERING.md`** — product shape, architecture, launcher contract, performance, privacy, non-goals.
2. **`docs/MODULES.md`** — user-visible module behavior.
3. **`docs/DECISIONS.md`** — compact active and historical decision log.
4. **`docs/QA.md`** — automated gates, manual smoke, recorded review, release checklist.

Historical dashboard/card routes are archaeology only. Do not plan against them unless the user explicitly asks.

## Reviewer Focus

- Route C alignment and handbook compliance.
- Finish quality over new surface area: permissions, empty states, cross-module flows, keyboard-first correctness.
- Push back on scope creep.

## Review Checklist

- Route C / handbook alignment
- Architecture boundary discipline (`LumaCore` UI-free; services in `LumaServices`)
- Hot-path safety (no disk/network I/O per keystroke; AX IPC off MainActor)
- Permission UX, persistence safety, secret/privacy handling
- Keyboard-only usability and visual consistency
- `LauncherEnvironment.showStatus` is non-optional `let` — inject at init
- Filesystem-heavy modules (Notes, Projects, MenuItems, Media) stay `onDemand`; not in `fastModuleIDs`
- Snippet trigger expansion only in global search; not on `s`/`snip` prefix queries

## Planning Output

For major planning requests, return: product judgment, current-focus recommendation, P0/P1/P2 priorities, risks, acceptance criteria, verification plan, docs to update.
