# Snippets + Secrets Implementation Plan

Status: complete (Snippets + Secrets shipped). ADR-010 accepted.
Route: B (Dashboard Widget Single Window) per ADR-007.
Date: 2026-06-22

---

## 1. Why these two

Goal: close more heavy apps on the daily machine.

| App replaced | Module | Example use |
| --- | --- | --- |
| TextExpander / Espanso / Alfred Snippets | **Snippets** | `git rebase -i HEAD~3`, docker one-liners |
| 1Password slice (dev creds only) | **Secrets** | API keys, SSH passphrases, DB passwords |

Both touch sensitive-ish data, but **Snippets and Secrets stay separate modules** — their security models are inverted (see §2).

---

## 2. Two-module positioning (final)

| Module | Priority | Effort | Dashboard | Storage | Security model |
| --- | --- | --- | --- | --- | --- |
| **Snippets** | P0 | 2–3 days | Yes — green card | JSON plaintext | Always available |
| **Secrets finish** | P0 | 1–2 days | Yes — gold card | Keychain | Locked by default; auto-lock after 5 min idle |

---

## 3. Merge rejection: Snippets ≠ Secrets

| Dimension | Secrets | Snippets |
| --- | --- | --- |
| Default state | Locked | Always open |
| Storage | Keychain | JSON plaintext |
| Reveal | Hidden until click | Always visible |
| Pasteboard | Auto-clear after 10 s | Persistent |
| Mental model | Paranoid vault | Lazy cheatsheet grab-and-paste |

Merging drags snippets into unlock friction (kills value) or strips secrets of locking (kills safety). **Do not merge.**

Snippets display name: **Snippets** (速记 / cheatsheet). Linux commands, git incantations, boilerplate — that's the product.

---

## 4. Module enable/disable UI

**Already implemented.** `SettingsWindowController` renders a checkbox per module from `BuiltInModules.makeAll()`. Todo and Wordbook appeared automatically after ADR-009; Snippets and Secrets will too once registered. Zero extra UI work — verify toggling off suppresses dashboard cards.

---

## 5. Module A: Snippets (P0) — shipped

### Scope

- Local-only plaintext snippet library: shell, git, code idioms, boilerplate.
- Trigger: `s ` (also `snip `).
- `s <query>` → fuzzy match over title + tags + content prefix; Return copies content.
- `s` alone → top 8 by frecency.
- Secondary action (Tab) → paste into front app via `AccessibilityClient.insert` (clipboard-only fallback when Accessibility is not granted).
- Detail view: Duplicate creates a titled copy (`<title> Copy`) via `SnippetsStore.duplicate(id:)`.
- Dashboard card: green gradient `#34C759 → #248A3D`, icon `text.cursor`, column 1 row 1.

### Files

```
Sources/LumaModules/Snippets/
  SnippetsModule.swift
  SnippetIndex.swift
  SnippetsStore.swift
  SnippetsAction.swift
Sources/LumaApp/Launcher/
  SnippetsDetailView.swift
Tests/LumaModulesTests/
  SnippetIndexTests.swift
  SnippetsStoreTests.swift
```

### Data model

```swift
public struct Snippet: Sendable, Codable, Hashable, Identifiable {
    public let id: UUID
    public var title: String
    public var content: String
    public var tags: [String]       // lowercased, deduped
    public var usageCount: Int
    public var lastUsedAt: Date
    public var createdAt: Date
}
```

Persistence: `~/Library/Application Support/Luma/snippets.json` — `{ "version": 1, "items": [...] }`.

### Frecency score

```text
score = fuzzyMatch × 0.5
      + log(1 + usageCount) / log(51) × 0.3
      + exp(-ageSeconds / 86400) × 0.2
```

Tag exact-match boost: +0.15 on fuzzy component. Tie-break: title lexicographic.

### Hot-path discipline

- Warmup: read JSON once → populate in-memory index.
- `handle`: memory only, never disk. p95 ≤ 20 ms warm.
- Edits: write via `SnippetsStore`, refresh index.

### Non-goals

System-wide text expansion, markdown/rich text, cloud sync, folders, `{{date}}` placeholders.

---

## 6. Module B: Secrets finish (P0) — shipped

### Current state

`Sources/LumaModules/Secrets/` exists (~280 lines): vault, Keychain store, unit tests green. **Not in `BuiltInModules.makeAll()`**, no dashboard card, no CRUD UI.

### Gaps

1. Wire into runtime + gold dashboard card (column 2 row 1, `#FFD60A → #FF9F0A`, `secret `).
2. `SecretsDetailView` — CRUD table; locked state shows Unlock button.
3. Auto-clear pasteboard after copy (10 s default, change-count guard).
4. Re-lock after 5 min idle; menu-bar 🔒/🔓 indicator.
5. Settings section: clear duration, lock timeout, require-unlock-on-launch.

### Non-goals

Browser password autofill, TOTP, team sync, breach monitoring, 1Password import.

---

## 7. Sequencing

| Step | Work | Days | Status |
| ---: | --- | ---: | --- |
| 1–2 | Snippets module + store + tests + detail view + card | 3 | Done |
| 3–5 | Secrets runtime + CRUD + auto-clear + 5 min lock | 2 | Done |

**Total ~5 days** focused work for this milestone.

---

## 8. Acceptance criteria

**Snippets:** tests green; `s git rebase` ≤ 20 ms warm; CRUD round-trip; Tab pastes via AX.

**Secrets:** card visible; lock/unlock cycle; 10 s pasteboard clear; 5 min auto-lock; menu-bar indicator.

---

## 9. Docs to update (with ADR-010)

- `docs/adr/010-snippets-secrets.md`
- `docs/ROADMAP.md` — Snippets + Secrets → v0.1
- `docs/NON_GOALS.md` — narrow secrets scope; add browser-autofill / TOTP exclusions
- `docs/specs/PERFORMANCE.md` — Snippets p95 ≤ 20 ms; Secrets p95 ≤ 30 ms
- `docs/MANUAL_QA_CHECKLIST.md` — per-module QA rows

---

## 10. Explicitly not doing

- Merge Secrets + Snippets
- Separate module-management UI (Settings checkboxes suffice)
- **CCSwitch / Claude Code multi-account switching** — dropped; depends on fragile internal credential format and unclear ToS
- AI Usage dashboard for API-billing customers
- Multi-vendor quota monitoring in v1
- System-wide auto-expansion triggers
- Cloud sync / account login
- Folder hierarchy in any new module

---

## 11. Next actions

1. Write ADR-010 locking Snippets + Secrets decisions (no CCSwitch).
2. TaskCreate remaining Secrets implementation tasks.
3. **Secrets finish** — wire runtime, CRUD UI, auto-clear, re-lock.
