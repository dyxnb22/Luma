# Launcher Home Constraints (Frozen)

**Status:** Enforced as of 2026-07-03.  
**Authority:** When home-screen behavior conflicts with older ADRs, PRD bullets, or `WORKBENCH_STRATEGY.md` “Home” examples, **this file wins** until a new ADR supersedes it.

## Intent

The empty-query launcher home is a **calm app switcher**, not a dashboard, inbox, or onboarding surface. Users reach everything else through **search**, **command prefixes**, or **module detail** — the same Raycast/Spotlight mental model.

Do **not** reintroduce home clutter “for discoverability” without an explicit ADR and QA pass.

---

## What Home Shows (Only)

| Section | `LauncherHomeSectionKind` | Source | Notes |
| --- | --- | --- | --- |
| **Open Apps** | `.openApps` | `OpenAppsHomeProvider` | Running regular apps, ordered by activation recency. |

That is the **only** section `LauncherHomeAggregator` may append on empty query.

### Open Apps rules

- Show **all** running apps — **no** `+N more` collapse row (`openApps.more`).
- `LauncherHomeCoordinator.snapshot()` always configures `appLimit: nil`.
- Do not list Luma itself.
- Multi-window apps may still expand/collapse per-app window rows (unchanged).
- Section header label comes from i18n key `home.section.openApps` (e.g. 打开应用 / Open Apps).

---

## What Home Must NOT Show (Frozen Out)

The following are **removed from empty-query home** and must **not** be wired back without a superseding ADR:

| Former surface | Former kind / provider | Why removed |
| --- | --- | --- |
| 开始使用 / Get Started | `.setup` / `SetupHomeProvider` | Cluttered home; permissions belong in Settings |
| 完成 Luma 设置 row | setup onboarding row | Same |
| First-run onboarding wizard | `OnboardingWizardDetailView` (deleted) | Auto-present blocked focus; user wants direct home |
| 最近 / Recent | `.recentActions` / `RecentActionsHomeProvider` | Duplicate of search + clipboard module |
| 继续 / Continue | `.continueFlow` / resume + contextual continue | Workbench continues via `proj`, module triggers, detail |
| 创建 / Create | `.create` / contextual create rows | Capture/converts via search + commands |
| `+N more` apps row | `OpenAppsResultBuilder.moreRow` | Hides apps behind extra step |

### Still allowed off-home

These features **remain product scope** but surface elsewhere:

- **Search results** — global query, module prefixes (`clip`, `t`, `word`, …).
- **Bare commands** — `todo`, `word review`, `app top`, module bare-open-detail, workbench `proj` / `cap` / `attach`.
- **Module detail** — same-panel `BaseDetailContainer` views.
- **Settings** — permissions, modules, language, retention.
- **Workbench** — activity/links stores, project workspace detail, command router (see `WORKBENCH_STRATEGY.md`).

`ContextualHomeProvider`, `ResumeHomeProvider`, and `HomeContributor` types may stay in the codebase for workbench context building, but **`LauncherHomeAggregator` must not render their rows on home**.

---

## Panel Geometry (Frozen)

See **[LAUNCHER_PANEL_CONSTRAINTS.md](LAUNCHER_PANEL_CONSTRAINTS.md)** for positioning rules, forbidden transforms, and module in-panel layout.

Authoritative tokens: `Sources/LumaCore/Design/LauncherChromeTokens.swift`  
Authoritative math: `Sources/LumaCore/Design/LauncherPanelGeometry.swift`

| Token | Value | Notes |
| --- | ---: | --- |
| `defaultPanelWidth` | **720** pt | Spotlight/Raycast-like width |
| `defaultPanelHeight` | **680** pt | Taller than pre-2026-07 wide shell |
| `minPanelWidth` / `maxPanelWidth` | 640 / 760 | |
| `minPanelHeight` / `maxPanelHeight` | 600 / 760 | |
| `panelVerticalBias` | **0.68** | Upper third of screen |

Do **not** revert to the historical **900 × 600** “wide dashboard” proportion without ADR.

Implementation: `LauncherWindowController.positionPanel` → `LauncherPanel.position(on:)` → `LauncherPanelGeometry.panelFrame`.

---

## Home List Visual Rules (Frozen)

- **Idle rows:** transparent background — no gray card fill on every row.
- **Hover:** subtle fill only (`listRowHoverAlpha` ≈ 0.06).
- **Selection:** accent fill + sidebar strip on `backgroundView` child; list row host (`NSControl`) must not use `wantsLayer`.
- **List container:** no gray inset card on `contentContainer` (`installHomeListSurface` is clear).
- **Section headers:** text + optional divider only — no gray capsule backgrounds.

Do not “add depth” by painting every row gray; that reads as disabled UI.

In-panel layout rules (search bar, list, detail) live in **[LAUNCHER_PANEL_CONSTRAINTS.md](LAUNCHER_PANEL_CONSTRAINTS.md)** — same freeze applies when changing `LauncherListRow` or home list chrome.

---

## i18n & Detail Chrome (Current)

- UI strings: `L10n` + `Sources/LumaCore/Resources/L10nStrings.json` (en + zh-Hans).
- Settings → General → Language: System / English / 简体中文.
- Module detail search placeholder: `translate.detail.placeholder` → “In %@ — Esc to go back” / “在 %@ 中 — Esc 返回”.
- Use `L10n.tr` with string literals (`StaticString`); never `String(describing: LocalizationValue)`.

---

## Code Touchpoints (Do Not Drift)

| Area | File | Constraint |
| --- | --- | --- |
| Home composition | `LauncherHomeAggregator.swift` | Only `.openApps` on empty query |
| App wiring | `AppCoordinator.swift` | No `SetupHomeProvider` on coordinator |
| Coordinator | `LauncherHomeCoordinator.swift` | `appLimit: nil`; no `showAllApps` collapse |
| Open apps | `OpenAppsHomeProvider.swift` | No `moreRow` append |
| Onboarding | — | No auto-present; wizard view removed |
| Tokens | `LauncherChromeTokens.swift` | Panel + list visual values above |
| Rows | `LauncherListRow.swift` | Idle = clear background; selection on `backgroundView` child only |
| Panel layout | `LAUNCHER_PANEL_CONSTRAINTS.md` | No full-width `wantsLayer`; stabilize after list transitions |

### PR checklist for home changes

Any PR that touches home aggregation, open-apps limits, setup/onboarding, home list chrome, or list-row layout must:

1. Cite an ADR if it **adds** a home section or row type.
2. Update this file and `docs/specs/UX_BEHAVIOR_RULES.md`.
3. Update `docs/MANUAL_QA_CHECKLIST.md` home section.
4. If list chrome touches full-width layout, also update `docs/specs/LAUNCHER_PANEL_CONSTRAINTS.md`.
5. Add/adjust tests in `LauncherHomeAggregatorTests` or `LauncherListRowsTests`.

---

## How to Ship New “Discoverability” Instead

| Need | Correct surface | Wrong surface |
| --- | --- | --- |
| Resume Wordbook | `word` / `word review` search | Home Continue row |
| Clipboard history | `clip` or global search | Home Recent row |
| Create todo from text | `t …` / Todo detail | Home Create row |
| First-run permissions | Settings → Permissions | Home setup section |
| Onboarding | Settings copy or one-time alert (if ever) | Auto wizard on launch |
| More running apps | Show all in Open Apps | `+N more` row |

---

## Related Docs

- [Launcher Panel Constraints](LAUNCHER_PANEL_CONSTRAINTS.md) — geometry, positioning, in-panel layout
- [UX Behavior Rules](UX_BEHAVIOR_RULES.md) — interaction contract
- [Launcher Navigation Audit](../qa/LAUNCHER_NAVIGATION_AUDIT.md) — temporary open-issue register (navigation, shortcuts, session)
- [ADR-023](../adr/023-command-first-unified-list.md) — Route C (amended 2026-07-03)
- [NON_GOALS](../NON_GOALS.md) — explicit non-goals including home clutter
- [MANUAL_QA_CHECKLIST](../MANUAL_QA_CHECKLIST.md) — regression checks
