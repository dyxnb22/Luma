# Launcher Show / Hide Entry Contract (Phase 12.2)

**Date:** 2026-07-07  
**Baseline:** `803b0672` + Phase 11–12 work  
**Canonical owner:** `LauncherWindowController` + `LauncherPanelVisibilitySession` (C-UI-001)

---

## 1. Entry table (code facts)

| User action | API path | Hidden-only guard | Debounce | Hide vs show | Session `panelHideBegan` | Notes |
|-------------|----------|-------------------|----------|--------------|--------------------------|-------|
| **Carbon hotkey** (global) | `HotkeyController` → `showFromCarbonHotkey()` → `show(reason: .carbonHotkey)` | **Yes** — no-op when `visibilitySession.isVisible` | **120 ms** `lastCarbonShowAt` | Show only | No | Does not hide when visible |
| **⌘Space while panel focused** | `LauncherPanel.performKeyEquivalent` → `hideFromVisibleHotkey()` | Visible-only | **120 ms** `lastPanelHideAt` | Hide only | Via `hide()` → `cancelActiveQueryAndSnapshotApply` | Also checks AppKit `panel.isVisible` |
| **Menu bar “Show Luma”** | `MenuBarController` → `showFromMenuBar()` → `show(reason: .menuBar)` | **No** — re-front/refocus when already visible | None | Show only | No | **Intentional** refocus path (Phase 12.3) |
| **Esc** (empty query, no detail) | `handleEscape` → `onDismiss` → `hide()` | N/A | None | Hide (fade) | Yes | Saves session in `finishHide` |
| **Action dismiss** | `onActionDismiss` → `hideImmediatelyForAction()` | N/A | None | Hide (immediate) | Yes | `resetForActionDismiss` or detail persist |
| **External app activation** | `hideIfShowingForExternalActivation` → `hideImmediatelyForAction()` | Skips self bundle | None | Hide | Yes | Then `refreshHome()` if was visible |
| **Settings open** | `hideImmediatelyForAction()` + `settingsWindowController.show()` | N/A | None | Hide launcher | Yes | Settings is separate window |
| **Diagnostics export** (menu) | No hide — async export only | N/A | N/A | N/A | No | Recovery entry does not require launcher visible |
| **Doctor** (menu) | No hide | N/A | N/A | N/A | No | Same |
| **`toggle()`** | `visibilitySession.isVisible ? hide() : show()` | No | **120 ms** `lastToggleAt` | Toggle | On hide | **No production caller found** in `AppCoordinator` |

---

## 2. `show(reason:)` contract (Phase 12.3)

```swift
public enum LauncherShowReason {
    case carbonHotkey  // hidden-only + carbon debounce
    case menuBar       // may show when already visible (refocus)
    case restore       // reserved — same visible policy as menuBar
    case qa            // reserved — same visible policy as menuBar
}
```

| Reason | `shouldBeginShowWhenAlreadyVisible` | Carbon debounce | User-visible behavior |
|--------|-------------------------------------|-----------------|------------------------|
| `.carbonHotkey` | false | yes | Open launcher from background app |
| `.menuBar` | true | no | Open or **re-front + refocus** search |
| `.restore` | true | no | Future session restore entry |
| `.qa` | true | no | QA smoke hooks |

**Core implementation:** `LauncherShowEntryPolicy` (LumaCore) + `LauncherWindowController.show(reason:)` routes to unguarded `show()` when allowed.

---

## 3. Intentional differences

| Difference | Intentional? | Rationale |
|------------|--------------|-----------|
| Menu bar bypasses hidden-only guard | **Yes** | User explicitly chose Show from menu; refocus is desired when launcher already visible |
| Menu bar bypasses Carbon 120 ms debounce | **Yes** | Debounce targets global hotkey double-fire, not explicit menu action |
| Carbon show does not toggle hide | **Yes** | Hide is ⌘Space on focused panel only (`PRODUCT_FLOWS` Flow 3) |
| `hide()` vs `hideImmediatelyForAction()` | **Yes** | Actions skip fade; Esc uses fade |
| Hide does not tear down detail | **Yes** | Session restore on re-show (`launcher-navigation` rule) |
| `toggle()` exists but unused in production | **Unknown** | Dead API or future entry — not part of MVP contract |

---

## 4. Needs convergence or tests

| Item | Status after Phase 12.3 |
|------|-------------------------|
| C-UI-001 menu bar bypass | **Documented + named entry** `showFromMenuBar()`; behavior unchanged |
| Unified `show(reason:)` | **Done** for carbon + menu bar |
| Visible menu bar Show regression | **Tested** — `LauncherMenuBarShowEntryTests` |
| `panelShowBegan` / `panelHideCompleted` session events | **Not wired** — see `LAUNCHER_SESSION_STATE_AUDIT.md` |
| Settings/Diagnostics hide behavior | **Unchanged** — documented above |

---

## 5. Hide cleanup chain (reference)

| Path | Async cancel | Detail prepare | Session save | Panel orderOut |
|------|--------------|----------------|--------------|----------------|
| `hide()` (fade) | `cancelActiveQueryAndSnapshotApply` | `prepareDetailForHide` | `saveCurrentSession` in `finishHide` | `finalizePanelHidden` |
| `hideImmediatelyForAction` | same | `prepareDetailForHide` | via `resetForActionDismiss` / detail persist | `finalizePanelHidden` |

---

## 6. Compliance signals

- `swift test --filter LauncherShowEntry`
- `swift test --filter LauncherMenuBar`
- `swift test --filter HotkeyDoubleFire`
- `docs/QA.md` P0 smoke: hotkey show/hide, menu bar Show

---

*Phase 12.2 — contract document. Phase 12.3 implements minimal `show(reason:)` routing.*
