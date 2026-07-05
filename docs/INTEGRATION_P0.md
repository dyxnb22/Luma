# Integration P0

Luma is broadly built. The task here is making existing flows feel complete, reliable, and fast — not adding surface area.

Status legend: ✅ done · 🔄 in progress · ☐ not started

## 1. Permissions

- ✅ Accessibility-denied states show an actionable path (permission banner with Settings link)
- ✅ Permission banners do not trap focus; Esc/back behavior is preserved
- ✅ `LauncherEnvironment.showStatus` is non-optional — permission state is always surfaced
- ✅ Automation-denied Browser Tabs flows degrade cleanly (actionable error, not raw AppleScript)
- ✅ Default-off modules explain why they are off and what enabling them implies (Settings → Modules)

## 2. Empty States

- ✅ Snippets detail shows empty state with "add one" prompt
- ✅ Todo shows "Nothing due today · Type a task to add it"
- ✅ Notes and Secrets detail empty states with next actions
- ✅ Media and Projects detail views show actionable empty states
- ✅ Empty search results clarify query vs missing data via `SearchEmptyState` hints
- ✅ First-run opens **Open Apps home directly** — no setup rows or auto-onboarding wizard (`LAUNCHER_HOME_CONSTRAINTS.md`)

## 3. Cross-Module Flows

- ✅ Clipboard → Snippet (save as snippet from clipboard detail)
- ✅ Clipboard / selection → Note (append to daily note action)
- ✅ Clipboard entry → inline expansion via snippet trigger
- ✅ URL / selection → Quicklink draft
- ✅ Current project context surfaces in **project workspace detail** and `proj` commands (not empty-query home)
- ✅ Records / Wordbook / Notes actions round-trip cleanly back into launcher search

## 4. Detail Return Chain

- ✅ Esc unwinds: action panel → detail → results/home → close
- ✅ Back button and Esc land in the same place
- ✅ Return from a detail-triggering row preserves a coherent session (Wordbook verified)
- ✅ Hide/show launcher preserves query/detail via `LauncherSessionStore`

## Done When

- ✅ Core permission flows are understandable without docs
- ✅ Empty states always suggest a next step
- ✅ Cross-module actions visibly land where the user expects (project context + resume rows)
- ✅ Detail navigation feels consistent across modules
- ✅ No P0/P1 regressions in the recorded review pass
