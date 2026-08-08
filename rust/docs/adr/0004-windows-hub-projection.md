# ADR-0004: Windows module + Hub projection

- Status: Accepted
- Date: 2026-07-14
- Last amended: 2026-07-26

## Context

Personal daily use needs a fast window switcher (~10 frontmost-app windows). ADR-0001
previously listed window search as a stub non-goal. That boundary is explicitly opened for
**list + focus only** (see ADR-0001 amendment). Hub “Pinned” (former Notes shortcuts / Clipboard
favorites) is retired from the empty-prompt Hub. Notes was subsequently retired; Clipboard
pin/unpin remains available via `/clip`.

## Decision

1. **Module `luma.windows`** — interactive trigger `/win` (aliases `/window` / `/windows`), `TargetedOnly`,
   default **on**. Lists visible windows; primary action `focus`.
2. **Hub projection** — empty prompt shows **all visible windows** (terminals / Luma
   filtered out), sorted by app then title. Enter focuses immediately (does not fill the
   prompt). Default cap **7** rows (`hub_windows_max`, clamped 5–50); overflow is a single
   `N more → /win` row that opens the full module. Row labels include `title · app` for
   disambiguation. When any title is `Untitled`, Hub status hints to grant Screen Recording.
3. **Hub pins removed** — empty-prompt Hub no longer shows former Notes shortcuts or Clipboard
   favorites. Notes was subsequently retired; Clipboard pin/unpin and purge-keeps-pinned remain
   inside `/clip`.
4. **Permissions** — list may lack titles without Screen Recording (`Untitled` / app name);
   focus needs Accessibility. Failures use `PermissionRequired` / `Unavailable`, never a
   silent empty list.
5. **Tests** — never call real `focus`, `osascript`, or otherwise steal focus (MODULES.md).
6. **Bounded Continue** — after Windows, Hub may show at most **3** recent Recall objects after
   the owning module revalidates them. Clipboard bodies and submitted search text remain excluded.

## Consequences

- Hub = Windows slice + bounded live/recent Continue + Modules (see MODULES.md).
- Out of scope: Window layouts, menu-bar search, Browser tabs, global hotkey overlay.
