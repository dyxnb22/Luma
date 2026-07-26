# Selected modules implementation contract

Status: implemented and inventory-synced. This document is now the durable feature boundary and
review checklist; availability is reported in `MODULES.md`.

This document turns the selected ideas into bounded Luma modules. The selected scope is:

1. Downloads Inbox
2. Packages
3. Calculator
4. Apple Shortcuts Bridge
5. Screen OCR
6. Shell Recall
7. Renewals
8. Database Portals

Containers are intentionally excluded. OrbStack already owns the container/VM experience; if
Luma later needs container awareness, it should be a small read-only provider inside the Projects
workbench rather than another top-level container manager.

All eight modules are registered and represented in both `MODULES.md` and the root README. Future
changes must keep those inventories synchronized as required by `GOVERNANCE.md`.

## 1. Product and architecture constraints

These are hard acceptance rules, not suggestions:

- Luma remains a keyboard-first local workbench. None of these modules may introduce LLM chat,
  agents, task planning, autonomous execution, a background daemon, or multi-session orchestration.
- Every interactive surface uses a leading `/`. Bare text remains global search.
- `rust/bins/luma/src/compose.rs` remains the sole composition root.
- Modules depend on application ports/repositories. macOS APIs, process spawning, filesystem
  mutation, and SQLite opening stay in platform/storage adapters.
- TUI `update` and `render` remain pure. Do not add module-specific protocol `Command`/`Event`
  variants or central engine dispatch arms.
- Permission, unavailable, not-configured, empty, failed, and cancelled are distinct local states.
  Do not restore a Doctor surface.
- The Swift native host stays unchanged unless a host lifecycle bug is proven. It must not own OCR,
  module UI, stores, or Engine startup.
- Cancellation must be checked before a mutation and again after any awaited discovery that
  precedes a mutation.
- Commands must use a program plus explicit argument vector. Never build interpolated shell text.
- Tests must not open Finder, steal focus, modify the real pasteboard, invoke real package changes,
  capture the real screen, or write a real shell history.
- New persistent data belongs under `~/Library/Application Support/LumaNext/`; tests use explicit
  temporary paths or the existing LumaNext test override.

## 2. Implementation and audit order

The modules landed as complete vertical slices in this order. Use the same order for a broad
regression audit; do not replace working slices with cross-module scaffolding.

| Order | Module | Why here |
| --- | --- | --- |
| 1 | Calculator | Pure and low-risk; establishes the module/search/action shape without new I/O |
| 2 | Downloads Inbox | First bounded filesystem port and recoverable mutation |
| 3 | Packages | Reuses the explicit subprocess style with structured JSON output |
| 4 | Apple Shortcuts Bridge | Similar subprocess boundary, with interactive execution semantics |
| 5 | Shell Recall | Bounded file parsing plus a deliberate privacy rule |
| 6 | Renewals | First new SQLite source of truth |
| 7 | Database Portals | Metadata persistence plus external-client and production-safety rules |
| 8 | Screen OCR | Last because native Vision and Screen Recording behavior have the most uncertainty |

When materially changing one of these modules:

1. Run focused tests for the touched crates.
2. Run the full verification set in section 13.
3. Update `MODULES.md` and the root README when the user-visible inventory changes.
4. Keep the tree green before starting the next module.

Keep commits bounded and never discard unrelated working-tree changes.

## 3. Common module slice

Create only the pieces the current module needs:

- `luma-application/src/ports/<feature>.rs` for host I/O contracts, or
  `ports/<feature>_repo.rs` for persistence contracts.
- A fake or memory implementation close to the port when it improves deterministic module tests.
- `luma-platform-macos/src/<feature>.rs` for macOS/filesystem/process adapters.
- `luma-storage/src/<feature>_store.rs` and an application SQLite repository adapter only when
  the feature owns persistent data.
- `luma-modules/src/<feature>.rs`, or a directory once a real second production file is needed.
- Exports in each crate `lib.rs`.
- Construction and registration only in `bins/luma/src/compose.rs`.
- CLI additions only when safe provisioning cannot be done in the TUI.

Use `with_deps(...)` constructors. Keep action payloads structured JSON containing stable IDs and
the minimum data needed to re-read or revalidate live state. Never trust display titles as action
identities.

Common result rules:

- Stable result IDs must be independent of display order.
- Informational rows use `noop` and an existing informational kind so they neither enter global
  results nor Recall.
- Safe actions use `ActionRisk::Safe`.
- Mutating or production-sensitive actions use `Confirm`/`Destructive` and set confirmation.
- A successful action should be returned only after the side effect is known to have succeeded.
- Previews are bounded and must not expose secrets.

## 4. Calculator

### Contract

| Field | Value |
| --- | --- |
| Module ID | `luma.calculator` |
| Triggers | `/calc`, `/calculate` |
| Default | on |
| Search mode | `GlobalContributing`, guarded by a strict expression detector |
| Persistence | none |

The strict global detector is mandatory. Calculator may contribute only when the entire bare query
looks like an expression or explicit conversion. Words, paths, project names, and ordinary global
search must never be swallowed by a permissive parser.

### MVP syntax

- Arithmetic: unary `+`/`-`, `+`, `-`, `*`, `/`, `^`, parentheses, and postfix `%`.
- Numeric input: ASCII decimal syntax, underscores between digits, maximum 256 characters.
- Data sizes: `B`, `KB`, `MB`, `GB`, `TB`, `KiB`, `MiB`, `GiB`, `TiB`.
- Simple units: temperature (`C`, `F`, `K`), duration (`ms`, `s`, `min`, `h`, `d`), and length
  (`mm`, `cm`, `m`, `km`, `in`, `ft`, `mi`).
- Conversion form: `<value> <unit> in <unit>`, for example `128 MiB in GiB`.
- Integer bases: `0x`, `0o`, `0b`, with `/calc base <value> <2|8|10|16>`.
- Date helpers: `/calc unix <seconds>` and `/calc date <YYYY-MM-DD> +/- <N>d`.

Currency conversion is out of scope because it requires mutable network data. Symbolic algebra,
graphing, arbitrary function execution, and JavaScript/Python/shell evaluation are also out.

### Implementation guidance

- Implement a small deterministic tokenizer and Pratt or shunting-yard parser in the module.
- Reuse the existing `chrono` dependency for dates. Do not add a parser crate unless the hand-written
  grammar becomes less safe or less testable.
- Limit token count, nesting depth, exponent magnitude, and output length.
- Reject division by zero, invalid dates, incompatible unit dimensions, overflow, `NaN`, and
  infinity with an honest `command_error` row.
- Keep display formatting deterministic and locale-independent. Trim insignificant trailing zeros,
  but preserve enough precision to make round trips unsurprising.
- Primary action: copy the result through `PasteboardPort`.
- Secondary actions: copy the full equation; switch between decimal/hex for integer results.

### Tests

- Operator precedence, associativity, unary operators, parentheses, and postfix percent.
- Invalid token, depth/length limits, divide-by-zero, overflow, and non-finite results.
- Every supported unit pair, including temperature offsets.
- Leap day, month boundary, negative timestamps, and invalid date input.
- A table proving the global detector rejects normal phrases, paths, versions, and package names.
- Cancellation before pasteboard mutation.

## 5. Downloads Inbox

### Contract

| Field | Value |
| --- | --- |
| Module ID | `luma.downloads` |
| Triggers | `/dl`, `/downloads` |
| Default | on |
| Search mode | `TargetedOnly` |
| Persistence | none |

MVP surfaces:

- `/dl` and `/dl recent`
- `/dl large`
- `/dl old 30d`
- `/dl type archive|image|video|document|installer`
- `/dl <query>`
- `/dl rename <result-id> | <new-name>`

Primary action opens the selected item. Secondary actions reveal it in Finder, copy its path,
and move it to Trash. Rename uses the explicit slash command because the current ActionPicker has
no text-entry contract; do not add a protocol form solely for this module. Permanent deletion,
background watching, duplicate detection, automatic cleanup, decompression, and
arbitrary-directory browsing are out of scope.

### Port and adapter

Add a `DownloadsPort` with structured operations roughly equivalent to:

- `list(filter, limit, cancel) -> Vec<DownloadEntry>`
- `resolve(id) -> DownloadEntry`
- `rename(id, new_name)`
- `trash(id)`

The default adapter is rooted at `~/Downloads`; its test constructor must accept an explicit root.
Opening/revealing and copying should reuse `OpenPathPort`/`PasteboardPort` where possible.

Safety requirements:

- Scan at most 500 direct children; do not recurse and do not follow symlinks.
- Use metadata size and modified time, with deterministic ordering and stable path-derived IDs.
- Treat invalid UTF-8 filenames losslessly enough to identify the file; never panic while rendering.
- Before rename/trash, resolve the payload again and ensure the canonical parent is still the
  configured Downloads root.
- Reject separators, `.`/`..`, NUL, empty names, and collisions when renaming.
- Trash must use a recoverable macOS Trash operation. Never call `rm`.
- Moving to Trash requires confirmation. Rename requires confirmation if it changes an extension.
- Files that disappear between search and action return an honest failure.

### Tests

- Filters and ordering over a temporary directory.
- Symlink, traversal, collision, disappearing file, invalid name, and item-cap behavior.
- Identity revalidation immediately before rename/trash.
- Fake actions proving tests never invoke Finder or the real Trash.
- Cancel before mutation and no success on a partial failure.

## 6. Packages

### Contract

| Field | Value |
| --- | --- |
| Module ID | `luma.packages` |
| Triggers | `/pkg`, `/packages`, `/brew` |
| Default | on |
| Search mode | `TargetedOnly` |
| Persistence | none |

Homebrew is the only MVP backend. Do not introduce a generic multi-package-manager framework before
a second real backend is requested.

MVP surfaces:

- `/pkg` and `/pkg installed`
- `/pkg outdated`
- `/pkg formulae`
- `/pkg casks`
- `/pkg search <query>`
- `/pkg info <name>`

### Port and adapter

Define a `PackageManagerPort` returning structured package records and explicit states:
available, not configured, unavailable, and command failure. The macOS adapter:

- Resolves `brew` without shell startup.
- Executes a direct program plus argument vector.
- Prefers documented machine-readable output such as `brew info --json=v2`.
- Applies a timeout, cancellation, output-byte cap, and item cap.
- Treats JSON fields as forward-compatible: ignore additions, reject missing required identity.
- May hold a short in-process cache, but never runs a background refresh.

Actions:

- Info/preview and copy homepage/name are safe.
- Install, upgrade, and uninstall suspend into `InteractiveTerminal` with exact arguments.
- Every mutating action requires confirmation; show whether the target is a formula or cask.
- Revalidate the exact package identity and backend availability before returning the terminal plan.

Do not implement automatic updates, `brew cleanup`, taps, services, mass upgrades, or remote package
metadata fetching in MVP. Missing Homebrew is `not_configured`, not an empty package list.

### Tests

- Fixture JSON for formulae/casks, installed/outdated status, and forward-added fields.
- Missing binary, timeout, nonzero exit, malformed JSON, oversized output, and cancellation.
- Exact argument-vector tests for install/upgrade/uninstall; no shell interpolation.
- Formula/cask name validation and confirmation enforcement.

Reference: [Homebrew Querying Brew](https://docs.brew.sh/Querying-Brew).

## 7. Apple Shortcuts Bridge

### Contract

| Field | Value |
| --- | --- |
| Module ID | `luma.shortcuts` |
| Triggers | `/sc`, `/shortcut`, `/shortcuts` |
| Default | on |
| Search mode | `TargetedOnly` |
| Persistence | none |

MVP surfaces:

- `/sc` lists shortcuts.
- `/sc <query>` filters shortcut names.
- `/sc folders` lists custom folders.
- `/sc folder <name>` lists shortcuts in a custom folder.

Primary action runs the shortcut. Secondary actions view it in Shortcuts and copy its name.

### Port and execution semantics

Add a `ShortcutsPort` for listing shortcuts/folders and resolving exact identities. The adapter uses
`/usr/bin/shortcuts` with direct arguments:

- `shortcuts list`
- `shortcuts list --folders`
- `shortcuts list -f <folder>`
- `shortcuts view <name>`

The default Run action returns `InteractiveTerminal { program: "/usr/bin/shortcuts",
args: ["run", exact_name], ... }`. This preserves user prompts and permissions without inventing a
new TUI protocol path. It also avoids a timeout falsely killing a legitimate interactive shortcut.

Clipboard/file input and captured output are a second increment, not MVP. Add them only after a
cancellable subprocess contract can distinguish interactive from non-interactive shortcuts.

Safety and honesty:

- Never execute a fuzzy display string; resolve an exact current shortcut name first.
- No shell interpolation and no URL-scheme fallback.
- Missing command is unavailable; zero shortcuts is not-configured/empty with a useful hint.
- Do not enumerate shortcuts during warmup.
- Shortcut-internal permissions remain Apple-owned prompts; Luma must not report success before the
  interactive process exits successfully.

### Tests

- List/folder parsing with spaces, Unicode, duplicate display names, and empty output.
- Exact argument vectors for list, folder list, run, and view.
- Missing binary, nonzero exit, cancellation, and stale shortcut identity.
- Interactive terminal contract through existing CLI/TUI blackbox coverage.

Reference:
[Apple: Run shortcuts from the command line](https://support.apple.com/guide/shortcuts-mac/apd455c82f02/mac).

## 8. Shell Recall

### Contract

| Field | Value |
| --- | --- |
| Module ID | `luma.shell_history` |
| Triggers | `/hist`, `/history` |
| Default | on |
| Search mode | `TargetedOnly` |
| Persistence | none |

MVP is a privacy-conscious, read-only view of zsh history:

- `/hist <query>`
- `/hist recent`

Primary action copies a command. Direct execution, editing shell history, background indexing,
cross-shell merging, current-directory filtering, sync, and automatic conversion to Command
Recipes are out of scope. Plain zsh history does not reliably record each command's working
directory, so the module must not infer one from a previous `cd`.

### Port and parser

Add a `ShellHistoryPort` whose system adapter reads `~/.zsh_history`; tests inject explicit paths.

- Read only the tail of at most 4 MiB.
- Parse at most 2,000 entries and cap one command at 8 KiB.
- Support zsh extended-history records (`: epoch:duration;command`) and bounded continuation lines.
- Tolerate incomplete final writes and lossy UTF-8 without panics.
- Never write, truncate, lock, or normalize the source file.
- Search scans the bounded in-memory result on demand; no watcher and no store.

### Privacy rule

Shell commands must not be copied verbatim into `recall.sqlite`. Extend the existing
`privacy_safe_title` policy in `luma-application/src/engine/recall.rs` so
`luma.shell_history` records a generic title such as `Shell history command`. This is the same
bounded privacy policy already used for Clipboard, Snippets, and SSH, not a new dispatch path.

Also suppress entries that look credential-bearing, including:

- assignments/exports whose key contains `TOKEN`, `SECRET`, `PASSWORD`, `PASSWD`, `API_KEY`, or
  `PRIVATE_KEY`;
- common authorization flags/headers;
- commands containing an obvious URL userinfo password;
- NUL-containing or oversized records.

Prefer false negatives in search availability (hide a suspicious row) over copying a credential
into Luma. Do not log filtered command bodies. An informational subtitle may say how many rows were
hidden without showing them.

### Tests

- Plain and extended zsh history, multiline continuation, truncated tail, Unicode, and size caps.
- Secret-pattern fixtures proving command text never appears in search output, logs, or Recall.
- Generic Recall title test in the application crate.
- Copy action cancellation and pasteboard failure.
- No execution action is exposed.

## 9. Renewals

### Contract

| Field | Value |
| --- | --- |
| Module ID | `luma.renewals` |
| Triggers | `/renew`, `/renewals` |
| Default | on |
| Search mode | `TargetedOnly` |
| Persistence | `renewals.sqlite` |

This is a recurring-payment/due-date ledger, not a reminder daemon or banking integration.

MVP surfaces:

- `/renew` upcoming active renewals.
- `/renew due`
- `/renew 30d`
- `/renew add NAME | YYYY-MM-DD | AMOUNT CURRENCY | monthly|quarterly|yearly|Nd`
- `/renew edit <id> ...`
- `/renew paid <id>`
- `/renew cancel <id>`
- `/renew delete <id>`
- `/renew backup`

### Data model

Use integer IDs and a schema equivalent to:

- `id`
- `name`
- `category`
- `amount_minor` nullable integer
- `currency` nullable uppercase three-letter code
- `cadence_kind` (`once`, `monthly`, `quarterly`, `yearly`, `custom_days`)
- `cadence_value` nullable positive integer
- `next_due_date` ISO local date
- `auto_renew` boolean
- `status` (`active`, `completed`, `cancelled`)
- `url` nullable
- `note` nullable bounded text
- `created_at`; `updated_at` is an opaque CAS version prefixed by the mutation timestamp

Never use floating point for money. The parser must know the currency minor-unit scale or reject
unsupported/ambiguous amounts; do not silently assume every currency has two decimals.

### Repository and behavior

- Add `RenewalsRepository` in application and `RenewalsStore` in storage, following the existing
  Timers/Records adapter pattern.
- Hard cap 1,000 rows; updates to existing rows remain allowed at capacity.
- `paid` is atomic: monthly/quarterly/yearly recurrence advances from the previous scheduled date,
  not from “today”; a one-time renewal becomes completed.
- End-of-month recurrence clamps to the last valid day while retaining the intended anchor day for
  later months.
- Cancel and delete require confirmation. Delete is permanent; cancel preserves history.
- No background notification, EventKit integration, email parsing, receipt scanning, or bank sync.
- Backup uses the existing SQLite `VACUUM INTO` plus atomic-rename convention under LumaNext
  `backups/`.
- If the store cannot open, register an unavailable module with the same ID/triggers; never let
  `/renew` fall back to global search.

### Tests

- Monthly anchors across February/leap year, quarterly/yearly recurrence, and custom days.
- Paid idempotency/transaction rollback, one-time completion, cancel/delete confirmation.
- Currency precision, invalid dates, caps, text limits, and stable ordering.
- Store round trip, schema initialization, reopen, concurrent update behavior, and backup.
- Unavailable-store registration in composition tests.

## 10. Database Portals

### Contract

| Field | Value |
| --- | --- |
| Module ID | `luma.databases` |
| Triggers | `/db`, `/database`, `/databases` |
| Default | off; explicitly enable in Settings before adding the first portal |
| Search mode | `TargetedOnly` |
| Persistence | `database_portals.sqlite` for non-secret metadata |

This module is a connection launcher and bounded schema browser. It is not a query editor, SQL
notebook, database server manager, migration runner, or replacement for TablePlus/DataGrip.

### Staged MVP

Stage A must be complete before Stage B:

1. SQLite portals: add/list/open/reveal, bounded table/index/schema preview, metadata backup.
2. PostgreSQL launcher profiles using the user's existing libpq authentication (`~/.pgpass`,
   certificate, Kerberos, or an interactive `psql` password prompt).

Do not implement Luma-managed PostgreSQL passwords in the first version. `InteractiveTerminal`
cannot carry a secret-safe environment contract, and passwords must never appear in argv, action
payloads, Recall, metadata SQLite, logs, or previews. A later credential design requires a focused
security decision and a dedicated Keychain namespace; do not reuse visible Secrets labels.

### Metadata model

- `id`
- `label`
- `kind` (`sqlite`, `postgres`)
- SQLite `path`, or PostgreSQL `host`, `port`, `database`, `username`
- `environment` (`local`, `development`, `staging`, `production`)
- `created_at`, `updated_at`

Do not store a DSN because DSNs often embed passwords. Do not auto-scan `.env`, project config,
shell history, browser storage, or running-process environments for connections.

Do not record a successful open before the interactive child actually starts and exits. The
current generic terminal handoff has no completion callback for Database Portals, so the MVP keeps
no `last_opened_at` bookkeeping.

### Surfaces and actions

- `/db`
- `/db add sqlite LABEL | PATH`
- `/db add postgres LABEL | HOST | PORT | DATABASE | USER`
- `/db tables <id>`
- `/db schema <id>`
- `/db remove <id>`
- `/db backup`

SQLite actions:

- Open CLI: `sqlite3 <canonical_path>` via `InteractiveTerminal`.
- Reveal file.
- Browse bounded table/index names and normalized DDL through a read-only SQLite connection.

PostgreSQL actions:

- Open CLI: `psql --host ... --port ... --username ... --dbname ...`.
- Rely on existing libpq auth or the terminal's own prompt.
- Do not perform background connection tests in MVP.

Production portals require confirmation before opening an interactive client. Removal requires
confirmation and deletes only Luma metadata, never the database file/server/database. Use at most
500 portals, 500 schema objects per preview, 256 KiB total schema input, and 16 KiB per displayed
DDL row.

Path and identity requirements:

- Canonicalize SQLite paths on add and again on action.
- Require an existing regular file; reject directories and path traversal.
- Revalidate the stored portal and current path immediately before opening.
- Action payload contains portal ID, never a password or stale full connection command.

Persistence follows the application repository + storage adapter pattern. Store-open failure must
register an unavailable module with the same triggers. Default-off behavior should follow the
explicit force-off pattern used by Secrets; enabling remains an explicit Settings/config action
and must not be toggled implicitly just because a portal row exists.

### Tests

- SQLite metadata CRUD, caps, canonical paths, missing files, symlinks, and removal semantics.
- Read-only schema enumeration and bounded DDL.
- Exact `sqlite3`/`psql` argument vectors with hostile labels/hosts to prove no interpolation.
- Production confirmation and stale-profile revalidation.
- Assertions that no password/DSN appears in rows, payloads, Recall, logs, or the metadata DB.
- Store unavailable and missing client binary states.

## 11. Screen OCR

### Contract

| Field | Value |
| --- | --- |
| Module ID | `luma.ocr` |
| Triggers | `/ocr` |
| Default | on |
| Search mode | `TargetedOnly` |
| Persistence | none |

MVP surfaces:

- `/ocr`: one row, “Select screen region and copy recognized text”.
- `/ocr file <path>` may be added only after region capture works, using the same recognition port.

There is no OCR history, screenshot gallery, background capture, cloud OCR, or global indexing.

### Adapter proof and ongoing boundary

The platform adapter was proven before registration. Preserve these checks when changing it:

1. Capture a selected region using the system capture UX.
2. Feed the resulting image to Apple's Vision `VNRecognizeTextRequest`.
3. Return bounded plain text and delete the temporary image.
4. Verify Chinese (`zh-Hans`, `zh-Hant`) and English on the target macOS version.

Keep native calls inside `luma-platform-macos`. Do not move OCR into the Swift PTY host. If a safe
Rust-to-Vision adapter cannot be implemented without destabilizing the host/build, stop this module
and document the concrete blocker; do not silently substitute a network service.

### Port and lifecycle

Define a `ScreenOcrPort` returning structured outcomes:

- recognized text;
- user cancelled selection;
- Screen Recording permission required;
- no text found;
- capture unavailable;
- recognition unavailable.

`required_capabilities` remains empty so warmup never disables the whole module or prompts for
Screen Recording. `/ocr` owns the permission-required row locally, following the Windows module's
honesty pattern.

Operational requirements:

- Use an unpredictable temporary path with owner-only permissions.
- Delete the screenshot on success, cancellation, and every error path.
- Never log the image path or recognized text at info/error level.
- Vision runs locally with accurate recognition; cap output at 256 KiB.
- Check cancellation before capture, between capture and recognition, and before pasteboard write.
- Copy through `PasteboardPort` only after recognition succeeds.
- If selection is cancelled, return `ActionOutcome::Cancelled`, not failed or success.

### Tests

- Module tests use a fake OCR port and fake pasteboard.
- Permission-required, cancelled, empty, unavailable, oversized text, and pasteboard failure.
- Temp-file cleanup tests in the adapter without real capture.
- A manual macOS smoke check for selection overlay, Screen Recording denial/grant, Chinese/English,
  and confirmation that the native host remains only a PTY host.

Reference:
[Apple Vision: Recognizing text in images](https://developer.apple.com/documentation/vision/recognizing-text-in-images).

## 12. Cross-module decisions and explicit non-goals

### Global search

Only Calculator may contribute globally, and only behind its strict detector. The other seven are
targeted-only to avoid flooding global search with private or dense operational data.

### Recall

- Shell Recall uses a generic privacy-safe title.
- Database Portal Recall uses a generic privacy-safe title; labels, DSNs, endpoints, and
  credentials do not enter Recall.
- OCR recognized text is never a result title and never enters Recall.
- Downloads may recall only the filename, not a copied file body.
- Calculator always uses a generic `Calculation` Recall title so input expressions are not copied
  into a second store.

If a module's natural successful action would make unsafe Recall metadata, extend the existing
central privacy-title policy. Do not add a new engine protocol or per-module dispatch arm.

### Containers and Projects

No top-level Containers module. A future Projects environment row may detect OrbStack/Docker
availability on demand and offer an external open command or read-only status. It must not manage
images, networks, or virtual machines and is outside this plan.

### No premature shared framework

Packages, Shortcuts, and Database Portals all launch processes, but that is not enough reason to
invent a plugin ABI or generic automation system. Reuse existing explicit subprocess and
`InteractiveTerminal` contracts. Extract a genuinely generic helper only after two implemented
call sites have identical timeout/output/cancellation needs.

## 13. Verification and completion

Focused tests should run after each logical edit. Before declaring a module change complete:

```bash
cd rust
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p luma --test cli_blackbox
./scripts/check_architecture.sh
```

Also run targeted manual smoke checks for the newly added module, but do not turn personal smoke
checks into release or soak infrastructure.

A module is complete only when:

- its targeted slash command resolves correctly;
- happy, empty/not-configured, unavailable/permission, failure, and cancellation paths are honest;
- unsafe actions confirm and revalidate identity;
- tests do not touch the real system;
- full verification is green;
- `MODULES.md` and root README inventory are updated in the same change;
- no unrelated refactor, deferred stub, Doctor surface, agent feature, or Swift product UI was
  introduced.
