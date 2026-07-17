# ADR-0004: Windows module + Hub projection

- Status: Accepted
- Date: 2026-07-14

## Context

Personal daily use needs a fast window switcher (~10 frontmost-app windows). ADR-0001
previously listed window search as a stub non-goal. That boundary is explicitly opened for
**list + focus only** (see ADR-0001 amendment). Hub “Pinned” (Notes shortcuts / Clipboard
favorites) is retired from the empty-prompt Hub; those flows stay available via `/n …` / `/clip`.

## Decision

1. **Module `luma.windows`** — interactive trigger `/win` (aliases `/window` / `/windows`), `TargetedOnly`,
   default **on**. Lists visible windows; primary action `focus`.
2. **Hub projection** — empty prompt shows **all visible windows** (terminals / Luma
   filtered out), sorted by app then title. Enter focuses immediately (does not fill the
   prompt). Default cap **15** rows (`hub_windows_max`, clamped 5–50); overflow is a single
   `N more → /win` row that opens the full module. Row labels include `title · app` for
   disambiguation. When any title is `Untitled`, Hub status hints to grant Screen Recording.
3. **Hub pins removed** — empty-prompt Hub no longer shows Notes shortcuts or Clipboard
   favorites. Clipboard pin/unpin and purge-keeps-pinned remain inside `/clip`. Notes shortcuts
   stay available via `/n …`.
4. **Permissions** — list may lack titles without Screen Recording (`Untitled` / app name);
   focus needs Accessibility. Failures use `PermissionRequired` / `Unavailable`, never a
   silent empty list.
5. **Tests** — never call real `focus`, `osascript`, or otherwise steal focus (MODULES.md).

## Consequences

- Hub = Windows slice + Modules (see MODULES.md).
- Out of scope: Window layouts, menu-bar search, Browser tabs, global hotkey overlay.
