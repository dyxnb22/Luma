# Integration P0

Luma is already broadly built. The next step is not more feature breadth; it is making existing flows feel complete, reliable, and fast.

## 1. Permissions

- Accessibility-denied states must always show an actionable path, never a silent failure or raw system wording.
- Automation-denied Browser Tabs flows must degrade cleanly and teach recovery.
- Permission banners must not trap focus or break Esc/back behavior.
- Default-off modules should explain why they are off and what enabling them implies.

## 2. Empty States

- Every detail view needs a useful empty state with a next action.
- Empty search results should clarify whether the issue is query, missing data, disabled module, or missing permission.
- First-run states should point to setup, not expose implementation details.

## 3. Cross-Module Flows

- Clipboard -> Snippet
- Clipboard / Translate -> Note
- URL / selection -> Quicklink draft
- Current project context -> Commands / Snippets / Quicklinks
- Records / Wordbook / Notes actions should round-trip cleanly back into launcher search and detail views

## 4. Detail Return Chain

- Esc must unwind consistently: action panel -> detail -> results/home -> close
- Back button and Esc must land in the same place
- Return from a detail-triggering row should preserve a coherent session
- Hide/show launcher should preserve or intentionally reset state, never do something ambiguous

## Done When

- Core permission flows are understandable without docs
- Empty states always suggest a next step
- Cross-module actions visibly land where the user expects
- Detail navigation feels consistent across modules
- No P0/P1 regressions appear in the recorded review pass
