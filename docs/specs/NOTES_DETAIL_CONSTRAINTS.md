# Notes Detail Constraints (Frozen)

Authoritative scope boundary: `docs/adr/019-notes-long-term-markdown-workspace.md`.  
On-disk layout: `docs/specs/NOTES_FORMAT.md`.  
Panel geometry and in-panel layout: `docs/specs/LAUNCHER_PANEL_CONSTRAINTS.md`.  
Navigation and keyboard pairing: `docs/specs/UX_BEHAVIOR_RULES.md`.

## Product boundary

Notes is a **local-first Markdown workspace manager**, not an in-app editor. Luma indexes, organizes, and dispatches; **Typora** (or `NSWorkspace.open` fallback) owns editing and rendering.

## Detail layout (top → bottom)

| Row | Contents |
| --- | --- |
| `topStrip` | Root path label; **+ Note** / **+ Folder** (Tree mode only); Expand / Collapse; `[Tree \| Map]`; gear |
| `chipBar` | Left: **Today**, **Recent**, **Pinned** — Right: **Outline**, **Browse**, **Inbox(n)** |
| `filterStrip` | Filename filter field (Tree mode only) |
| Body | Outline `NSScrollView` or Mind Map `NSScrollView` |

`topStrip`, `chipBar`, `filterStrip`, and scroll views pin to the detail container width — never widen the panel.

## Left chips (quick views)

| Chip | Behavior |
| --- | --- |
| **Today** | Quick action only — opens or creates today's daily note in Typora; chip does **not** stay selected. Label `Today +` when today's file is missing. |
| **Recent** | Flat list of last opened notes (max 8, from `notes.json`). |
| **Pinned** | Flat list of notes with `pinned: true` frontmatter. |

**Do NOT** restore without a new ADR:

- An **Inbox** chip on the left (Inbox is a right-panel segment only).
- A **Today** chip that toggles a persistent filtered view.
- An embedded **Recent** group inside the Outline tree.

## Right panels

| Panel | Contents |
| --- | --- |
| **Outline** | Directory tree only — folders and `.md` files from `NotesTreeIndex`. |
| **Browse** | Grouped retrieval: "Modified this week" plus one group per distinct frontmatter `type`. |
| **Inbox** | Flat list of notes in the configured inbox folder; segment label `Inbox(n)` when `n > 0`. |

Selecting a left chip forces the right panel to **Outline** for display. Panel choice is independent when no chip is active.

## Create flows

Creation is explicit, reversible (Trash), and always opens new **notes** in Typora after create.

| Entry | Action |
| --- | --- |
| Toolbar **+ Note** | Create-note sheet; optional template picker when `_templates/` has files. |
| Toolbar **+ Folder** | Name prompt for new folder. |
| Context menu | New Note / New Folder on folder or empty tree — uses right-clicked row as parent context. |
| `⌘N` / `⌘⇧N` | Same as toolbar (outline mode; `topStrip` visible). |
| `n new <title>` | Launcher command — creates in Inbox and opens. |
| `n new <template> <title>` | Creates from named template in Inbox. |

**Default parent folder** (toolbar, shortcuts, context menu):

1. Selected folder
2. Parent of selected note
3. Inbox folder (created if missing)
4. Vault root

Launcher `n new` always targets **Inbox** (command capture path).

## Tree vs Map

- `[Tree \| Map]` toggles in-panel; no modal sheet.
- **+ Note**, **+ Folder**, filter field, Expand/Collapse: **Tree only** (hidden in Map).
- Map uses the same `NotesTreeIndex`; double-click note opens Typora.
- Esc exits launcher detail per global navigation rules — no sheet trap.

## Keyboard (detail, outline focused)

| Shortcut | Action |
| --- | --- |
| `⌘1` | Today quick action |
| `⌘2` | Recent chip view |
| `⌘3` | Pinned chip view |
| `⌘N` | New note |
| `⌘⇧N` | New folder |
| `⌘L` | Find backlinks for selected note |
| `⌘↩` | Open selected note in Typora |
| `F2` / `⌘R` | Rename |
| `⌘⌫` | Delete (Trash) |

Module shortcuts must remain reachable when outline or map holds focus (`handleKeyDown` on `NotesDetailView`).

## Scroll

Outline and Mind Map use `GeekUIKit.wireVerticalListScroll` / `syncVerticalListDocumentFrame`. Partial tree expand must show a vertical scrollbar when rows overflow — not only after Expand All.

## Expansion persistence

Outline expansion state saves to `notes.json` `expandedFolders` when:

- Panel hides with Outline active, no chip, empty filter
- Detail deactivates under the same conditions

Virtual paths (`__…`) are never persisted.

## PR requirements

When changing Notes detail IA, chips, panels, create flows, or shortcuts:

- Update this file and `docs/specs/UX_BEHAVIOR_RULES.md`
- Update `Features/Notes/README.md` and `docs/MANUAL_QA_CHECKLIST.md` Notes section
- Extend `Tests/LumaModulesTests/Notes*` when behavior is testable without AppKit
