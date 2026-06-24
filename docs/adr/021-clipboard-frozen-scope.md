# ADR-021: Clipboard v1 Frozen Scope

## Status

Accepted (2026-06-24)

## Context

Clipboard history is past MVP: local JSON persistence, retention caps, pin/delete/clear, image capture, source-app metadata, pasteboard-type privacy filtering, launcher `clip` query, and in-panel detail with search and keyboard shortcuts. Remaining work is **freezing** the data model, privacy rules, keyboard flow, and light UI—not building a Paste-style organization system.

References: Raycast Clipboard History (retention, pin/delete, save-as-snippet); Maccy (local, keyboard-first); Paste (explicitly out of scope for most features).

## Decision

### Product definition

Luma Clipboard is a **local, fast, private, keyboard-first** personal clipboard history. Users copy as usual; Luma captures in the background (when enabled), surfaces history via the Clipboard card or `clip ` in the launcher, and supports copy/paste/pin/delete from the detail panel.

### Frozen pillars

| Area | Frozen choice |
| --- | --- |
| Storage | `~/Library/Application Support/Luma/clipboard-history.json` |
| Persistence | JSON `ClipboardHistorySnapshot` (`schemaVersion` + `entries`); legacy bare `[ClipboardEntry]` array loads with migration |
| Entry kinds | `text`, `link`, `email`, `code`, `image`, `file`, `color` — detect + display + copy back only; no file previewer or color library |
| Dedup | `contentHash` per entry (text / image bytes / file URL set); replaces raw `text == text` / `imageData == imageData` |
| Retention | OR caps: max entries (default 500), max age (default 7 days), max text bytes (default 100 KB); pinned exempt from age/count prune |
| Privacy | Pasteboard concealed/transient types; password-manager type prefixes; **source bundle blocklist** (built-in + user ignore list); **conservative sensitive-text heuristic** (SSH keys, JWT, API-key patterns, Luhn cards) |
| Capture | 1s `NSPasteboard` poll; skip when source is Luma; priority: **file URL → image → text** |
| Search | Linear token match over `text`, `sourceAppName`, `detectedKind`, `fileURLs`, `colorHex`; optional prefix filters (`img:`, `link:`, `code:`, `file:`, `color:`) |
| UI | Single list (no grid wall); segments **All / Pinned / Image**; fixed row height; type icon or thumbnail; hover actions |
| Copy/paste | Unified via `ClipboardModule` + `PasteboardClient` (not direct `NSPasteboard` in views) |
| Launcher | Keyword `clip`; primary action **Copy** (Raycast-style copy/paste separation) |

### Core actions (frozen set)

| Action | Notes |
| --- | --- |
| Copy | Write entry back to system pasteboard |
| Paste | Copy + hide launcher + AX insert (text only; images/files copy-only) |
| Pin / Unpin | Pinned sort first; exempt from retention prune |
| Delete | Single entry |
| Clear Unpinned | Confirm; keeps pinned |
| Clear Recent… | 5 min / 1 h / today; unpinned only |
| Save as Snippet | Opens snippet editor sheet |
| Copy as Plain Text | Text entries only; explicit plain string write |

### Settings keys (frozen set)

- `clipboardMaxEntries`, `clipboardMaxAgeDays`, `clipboardMaxEntrySizeKB` (retention)
- `clipboardHistoryEnabled` (default **true**; stops capture polling when false)
- `clipboardIgnoredBundleIDs` (string array; merged with built-in password-manager / keychain bundles)
- `clipboardPasteBehavior` (`pasteDirectly` \| `copyOnly`; default `pasteDirectly`)

### Built-in source bundle blocklist (non-exhaustive)

1Password, Bitwarden, Dashlane, Keeper, Apple Keychain Access, plus user `clipboardIgnoredBundleIDs`. Banking apps: user-extensible via ignore list; no open-ended bundle scraping.

### Schema fields per `ClipboardEntry` (frozen)

`id`, `text`, `createdAt`, `isPinned`, `detectedKind`, `sourceAppName`, `sourceBundleID`, `imageData`, `imagePasteboardType`, `fileURLs`, `colorHex`, `contentHash`

New fields require ADR amendment.

## Non-goals (require new ADR to revisit)

1. OCR on images
2. Cloud sync, accounts, multi-device
3. Pinboard / multiple named boards
4. Rich-text editor or RTF storage
5. File previewer, color palette / library
6. Rename entries, bulk multi-select queue
7. Complex rule editor (regex, per-app rules beyond ignore list)
8. SQLite or FTS index (500-entry linear search is sufficient)
9. Grid / card-wall UI (Paste-style)
10. Streaming or webhook integrations

## Consequences

- Clipboard PRs labeled `feature` must link a new ADR if they touch frozen pillars above.
- Bugfix PRs may adjust filter lists, layout, copy, edge cases, performance, and tests without ADR.
- Privacy heuristics stay **conservative** (prefer not storing over false negatives).

## References

- `Sources/LumaModules/Clipboard/`
- `docs/NON_GOALS.md` § Clipboard Module
- ADR-004 (in-process modules), ADR-007 (dashboard widget)
