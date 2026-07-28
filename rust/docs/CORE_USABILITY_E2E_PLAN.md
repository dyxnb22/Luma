# Luma Core Usability End-to-End Review Plan

Status: executed; final user-authenticated permission/cleanup steps pending
Created: 2026-07-28
Last execution update: 2026-07-28
Scope owner: user + Codex
Target app: `/Users/diaoyuxuan/Applications/Luma.app`

This is the execution checklist for a complete usability review of Luma. It is not a product
roadmap, release checklist, soak-test plan, or centralized diagnostics design. Update the
checkboxes and evidence tables while executing the review so that no declared command, action,
state, or external effect is silently skipped.

Authoritative boundaries:

- `/Users/diaoyuxuan/Luma/AGENTS.md`
- `/Users/diaoyuxuan/Luma/rust/docs/GOVERNANCE.md`
- `/Users/diaoyuxuan/Luma/rust/docs/MODULES.md`
- `/Users/diaoyuxuan/Luma/rust/bins/luma/src/compose.rs`

## Execution record (2026-07-28)

This run used the isolated root `/tmp/luma-e2e.wFuQbv`, plus the final installed bundle for
native-window checks. Sensitive clipboard, Keychain, proxy, SSH, and database values are
deliberately omitted. The source inventory remained exactly 23 production modules and 97
declared `CommandSpec` rows.

| Area | Status | Evidence |
| --- | --- | --- |
| Native activation/lifecycle/input | `FIXED` | Installed app accepted leading `1`; hide/show retained `hide-state-1` and accepted the following `2`; one host and one bundled TUI remained alive while hidden. |
| Keyboard/render/PTY | `FIXED` | Real PTY blackbox covered digits, arrows, bracketed paste, CJK/emoji, PageUp/PageDown, and Ctrl-K; installed UI covered side/stacked preview resizing and Unicode paste; Swift terminal-filter tests covered split UTF-8 and control strings. |
| Windows/TCC | `FIXED` / `BLOCKED` | After final rebuild, Screen Recording remained authorized and real Chinese/English titles were visible without another prompt. Focus correctly reported the still-missing Accessibility permission; final focus success awaits the user-authenticated toggle. |
| Apps/Calculator/Downloads | `FIXED` | Calculator launch, Finder reveal, copy/restore, Downloads rename confirmation, real Finder Trash, three Put Back cycles, and pure JSON output passed. |
| Packages/Proxy/Runtime | `FIXED` | Test-owned Homebrew formula install/upgrade/uninstall, proxy enable/conflict/disable with exact state restoration, and exact test listener termination passed. |
| Shortcuts/Secrets | `PASS` / `BLOCKED` | Unicode exact-name Shortcut list/view/run/folder/copy passed; Keychain write/delete passed, while read required interactive system authentication. Test Shortcut deletion awaits action-time confirmation. |
| SSH/Databases | `FIXED` | Ephemeral sshd SSH/SFTP plus metadata actions passed with explicit config; SQLite and ephemeral PostgreSQL open/backup/remove passed; dependencies and processes were restored/stopped. |
| Clipboard/Quicklinks/Snippets | `FIXED` | Real clipboard capture pause/resume/pin/delete/clear and CRUD/overwrite/backup/copy/import paths passed; clipboard was restored. |
| Renewals/Timers/Wordbook/Records | `FIXED` | CRUD, confirmation/cancel, recurrence, review/mastery/import/backup, rating/note/top/unrated and migration rollback paths passed with isolated stores. |
| Projects/Git/Recipes | `FIXED` | Imported-project workbench/files, stage/unstage/discard/branch/log/commit, recipe run/copy/favorite and project index removal passed; project directory hashes were preserved. |
| OCR | `PASS` / `BLOCKED` | Permission preflight and cancellation cleaned the screencapture process/private temp file. Computer Use cannot deliver a drag to the system-owned crosshair, so final successful region recognition requires one user drag. |
| Verification | `PASS` | `cargo fmt`, `git diff --check`, architecture allowlist, workspace check, Clippy `-D warnings`, 719 Rust tests, 58 Swift tests, release build, deep codesign, bundle identity and embedded release UUID all passed. |

### Reproducible defects fixed in this run

| ID | Reproduction and root cause | Fix and regression evidence |
| --- | --- | --- |
| `UX-001` | Digits were consumed by Hub/Windows shortcuts outside the intended list surface. | Scope digit routing to the active surface/prompt; reducer tests plus installed-app leading-`1` and hide/show `2` retest. |
| `UX-002` | PTY filtering could treat UTF-8 continuation bytes as C1 controls or retain incomplete control state across sessions. | Make filtering UTF-8-aware and reset state per session; exhaustive two-chunk Swift tests and real PTY CJK/emoji coverage. |
| `UX-003` | Overlay/async replacement could leave old glyphs, and backspace could split visible graphemes. | Clear overlay cells, replace loading rows atomically, and edit by grapheme cluster; render/reducer regression tests and real resize/paste checks. |
| `UX-004` | Homebrew reads could trigger global updates, one-sided formula/cask misses hid valid results, and Homebrew 6 cask outdated JSON was rejected. | Disable auto-update for reads, isolate bounded queries, merge one-sided results, parse current schema, and harden exact mutation vectors; unit tests plus test-formula lifecycle. |
| `UX-005` | Finder Trash AppleScript could not resolve the argv path, then Finder stdout corrupted `--json`. | Bind `POSIX file` to `targetFile` and silence Finder subprocess stdout/stderr; adapter test plus real Trash/Put Back and JSON parse. |
| `UX-006` | SSH/SFTP resolution could use a different config from the eventual connection. | Carry the explicit `-F` config through resolver, TUI terminal plan, and CLI; argument tests plus ephemeral sshd SSH/SFTP. |
| `UX-007` | PostgreSQL was reported absent when only a versioned PATH binary or Homebrew keg-only `opt` binary existed. | Add deterministic executable discovery across plain, versioned, and Homebrew paths; resolver tests plus ephemeral PostgreSQL portal. |
| `UX-008` | Empty Records stores could not add their first record because no category existed; previews printed `Some(9)`. | Atomically create the category on first insert and format optional ratings for people; storage/module tests plus isolated CRUD. |
| `UX-009` | Imported projects were not browse roots unless separately duplicated in `projects_roots`. | Union imported project paths into the bounded browse roots while retaining escape checks; test plus real imported-project browse. |
| `UX-010` | Git workbench/branch/log rows returned not-found instead of opening their declared surfaces. | Return explicit `OpenSurface` routes after fresh imported-repo validation; module tests plus real repo navigation and commit. |
| `UX-011` | Timer names were lowercased because parsing used normalized query text. | Parse names from raw input while matching keywords case-insensitively; CJK/emoji tests and isolated timer lifecycle. |
| `UX-012` | Recipe JSON preflight errors emitted mixed/non-machine-readable output and copy used retired bare command syntax. | Emit pure JSON failures and route copy through `/cmd`; CLI blackbox/unit tests plus real recipe run/copy. |

### Remaining action-time handoffs

- `BLOCKED`: enable Luma under System Settings -> Privacy & Security -> Accessibility, then retry
  a `/win` Focus action. This is separate from the already-working Screen Recording grant.
- `BLOCKED`: perform one physical drag for `/ocr`; Computer Use synthetic mouse events are handled
  by SwiftTerm instead of the system-owned screencapture crosshair.
- `BLOCKED`: approve system authentication for one namespaced Keychain read if the real read path
  must be completed; the exact test entry will then be deleted again.
- `BLOCKED`: confirm permanent deletion of the test-created Shortcut
  `Luma E2E 文本 🙂 20260728`.
- `BLOCKED`: confirm permanent deletion of the recoverable test root
  `luma-e2e.wFuQbv`, which has already been moved from `/tmp` to the macOS Trash.

## 1. Product boundary

In scope:

- The 23 production modules registered by the Rust composition root.
- Every declared trigger, command specification, result type, primary action, secondary action,
  confirmation, cancellation, and visible failure state.
- Global search, Recall, Hub Continue, settings, help, command palette, overlays, and keyboard
  navigation.
- CLI-only bootstrap, import, configuration, and maintenance entry points.
- Rust TUI, Engine, platform adapters, persistence, PTY behavior, and the thin Swift native host.
- Actual build and signing of `/Users/diaoyuxuan/Applications/Luma.app`.
- Real macOS UI verification through Computer Use.
- Authorized external effects listed in section 5, using exact targets and rollback checks.

Out of scope:

- AI/LLM chat, autonomous agents, background orchestration, or multi-session agent behavior.
- A centralized doctor, diagnostics export, probe-port system, or Doctor overlay.
- Deferred Window Layouts, Menu search, Browser tabs, or other removed modules.
- Public release packaging, notarization, DMG creation, updater work, or marketing.
- Architecture-only cleanup without a reproduced personal-use problem.
- Destructive testing against arbitrary user data merely to satisfy a checkbox.

Test-only `luma.fake` remains covered by automated CLI tests but is not a production UI module.

## 2. Inventory baseline

Baseline observed when this plan was created:

- 23 production module rows in `MODULES.md`.
- 97 declared module command specifications.
- 40 TUI message families.
- 686 discoverable Rust workspace tests.
- Native host tests are maintained separately in the Swift package.

The test count is only the creation-time snapshot and is expected to change as fixes land. The
module and command counts are an inventory contract: any change must be explained by an intended
manifest/composition change rather than accepted as incidental drift.

- [ ] Record `git status`, branch, HEAD, recent commits, and existing user changes.
- [ ] Compare `MODULES.md`, root README module inventory, and `compose.rs`.
- [ ] Confirm the production composition root exposes exactly 23 modules and the per-module
      `CommandSpec` counts below (97 total):

      | Module | Declared `CommandSpec` count |
      | --- | ---: |
      | Apps | 1 |
      | Calculator | 1 |
      | Downloads Inbox | 4 |
      | Packages | 6 |
      | Apple Shortcuts | 3 |
      | Shell Recall | 1 |
      | Renewals | 7 |
      | Database Portals | 7 |
      | Screen OCR | 1 |
      | Windows | 1 |
      | Git | 5 |
      | Runtime | 1 |
      | Proxy | 9 |
      | Clipboard | 5 |
      | Quicklinks | 3 |
      | Snippets | 4 |
      | Wordbook | 9 |
      | Records | 11 |
      | Projects | 5 |
      | Command Recipes | 3 |
      | SSH | 5 |
      | Timers | 4 |
      | Secrets | 1 |

- [ ] Enumerate enabled and disabled manifests from an isolated Engine
      `Event::SessionReady.modules` payload backed by `registry.list_module_info()`; do not treat
      `luma modules list --json` as a complete manifest dump because it exposes only id, enabled,
      and display name.
- [ ] Compare the exact manifest id, display name, enabled state, triggers, suggested query,
      `CommandSpec` syntax/query/example, and global-search flag against `compose.rs`,
      `MODULES.md`, `/commands`, `/help`, and completion.
- [ ] Enumerate result kinds and primary/secondary actions from populated fixtures.
- [ ] For every fixture result, send `Command::ListActions` and collect
      `Event::ActionsAvailable`; union those action ids with each `SearchItemDto` primary action,
      secondary actions, and `ui_intent`.
- [ ] Include action paths that are not discoverable from one static search result: Wordbook
      review keys, recipe shortcuts, project `OpenSurface` routes, and SSH post-session
      `record_connection`.
- [ ] For every visible status/result row whose only action is `noop`, execute `noop` and verify
      that it makes no mutation, opens no surface, and creates no Recall entry.
- [ ] Enumerate CLI commands and nested subcommands from `--help`.
- [ ] Add a test-only inventory/parity test using Clap `CommandFactory` plus an isolated Engine;
      keep this as a test assertion and do not add a product `doctor`, diagnostics overlay, or
      runtime probe.
- [ ] Fail inventory/parity verification on a manifest/parser/palette/help mismatch, an
      unclassified action id or `ui_intent`, an undocumented parser alias, or a production module
      missing from any authoritative inventory.
- [ ] Record the installed app path, executable paths, bundle identifier, signature requirement,
      CDHash, and active host/TUI PIDs.
- [ ] Record current Screen Recording and Accessibility behavior without changing settings.
- [ ] Record tool and dependency availability: Homebrew, git, ssh, sftp, psql, lsof, shortcuts,
      screencapture, Vision/OCR, editors, and proxy controller.

### 2.1 Known command-discovery drift to resolve

These are acceptance items, not approved new product syntax. For each row, make the parser,
manifest `CommandSpec`, `/commands`, `/help`, completion, and tests agree; remove or expose an
alias according to the established leading-slash command contract.

- [ ] Projects: `/proj import PATH` is accepted by the parser and documented, while its manifest
      advertises only `/proj add PATH`.
- [ ] Records: `/rec ls [category]` is accepted as a hidden alias for browse.
- [ ] Timers: `/tm start [name]`, `/tm stopwatch [name]`, `/tm pomodoro ...`,
      `/tm cd ...`, and `/tm countdown ...` are accepted by the parser, while the manifest
      advertises only `pomo`, numeric countdown, and `sw` forms.
- [ ] Verify all non-module command-palette rows individually:
      `/settings`, `/settings projects-root <path>`,
      `/settings import-project <path>`, `/settings records-root <path|none>`,
      `/settings clipboard-retention-days <days>`,
      `/settings secrets-idle-lock-secs <seconds>`,
      `/settings hub-windows-max <5-50>`, `/scroll up`, `/scroll down`, `/help`,
      `/commands [filter]`, and `/quit`.
- [ ] Confirm unprefixed prompt text remains global search and that retired bare-trigger and colon
      forms are rejected while every accepted slash form is discoverable.

## 3. Result vocabulary

Every coverage cell must end in exactly one status:

| Status | Meaning |
| --- | --- |
| `PASS` | The real path and expected effect completed successfully. |
| `FIXED` | A reproducible defect was repaired, tested, rebuilt, and passed real UI retest. |
| `BLOCKED` | A required target, credential, authentication step, or OS state is unavailable. |
| `BOUNDARY` | The behavior is deliberately unsupported and Luma reports that honestly. |

Do not use “covered by unit tests” as the final status for a user-visible end-to-end cell.

For every command/action, cover the applicable states:

- [ ] Discovery through command palette/help/completion.
- [ ] Primary trigger and every alias.
- [ ] Empty or first-run state.
- [ ] Populated success state.
- [ ] Invalid input and boundary input.
- [ ] Not configured.
- [ ] Dependency unavailable.
- [ ] Permission denied.
- [ ] Permission granted.
- [ ] Confirmation shown before mutation.
- [ ] Cancellation causes no mutation.
- [ ] Successful action produces the stated effect.
- [ ] Failed action never reports success.
- [ ] Stale identity/result is revalidated.
- [ ] Rapid repeat and cancellation do not leak late results.
- [ ] State is correct after hide/show, TUI restart, app restart, and rebuild where applicable.

## 4. Test lanes and data isolation

### 4.1 Automated and contract lane

- Pure parser, reducer, rendering, persistence, cancellation, capacity, and adapter tests.
- CLI blackbox tests with isolated environment variables.
- Swift host policy, terminal filtering, geometry, lifecycle, and single-instance tests.
- Architecture allowlist, formatting, Clippy, plist, and code-signing verification.

### 4.2 Isolated real-process lane

Use a unique test root for every review run:

- `LUMA_NEXT_SUPPORT_DIR=<temporary test root>/support`
- `LUMA_NEXT_LOGS_DIR=<temporary test root>/logs`
- A test HOME where adapters honor HOME.
- A test PATH containing only explicitly selected real tools or test-owned shims.
- Generated Git repositories, SQLite databases, history files, SSH configuration, project
  directories, downloads, records imports, Wordbook imports, and recipes.

Run the actual built `luma tui` inside a real PTY for mutation-heavy scenarios. Do not point these
tests at the user's live LumaNext databases.

### 4.3 Final installed-app lane

Use `/Users/diaoyuxuan/Applications/Luma.app` for:

- Native window, PTY, activation, focus, lifecycle, input method, and rendering checks.
- Read-only or low-risk live macOS integrations.
- Test-owned external resources where the native adapter cannot use the isolated root.
- Final post-fix and post-rebuild verification.

The installed host intentionally filters test-only environment variables. Do not commit a hidden
test launch flag or weaken `ChildEnvironment` to make testing easier.

### 4.4 Test asset and rollback ledger

Create a temporary ledger before any external mutation:

| Asset | Exact identifier/path | Pre-state captured | Created by test | Cleanup/restore | Verified |
| --- | --- | --- | --- | --- | --- |
| Test root | TBD | N/A | yes | remove test root | pending |
| Clipboard | no content in report | yes | no | restore original | pending |
| Downloads fixture | TBD | N/A | yes | Trash/remove test fixture | pending |
| TCP listener | TBD PID/port | N/A | yes | terminate exact PID | pending |
| Homebrew tap/formula | TBD | yes | yes | uninstall/untap | pending |
| System proxy | active service + all fields | yes | no | restore exact snapshot | pending |
| Shortcut | exact name | yes | maybe | undo test-created item if any | pending |
| Keychain account | namespaced label | yes | yes | delete exact account | pending |
| SSH server/config | TBD | N/A | yes | stop/remove fixture | pending |
| PostgreSQL cluster | TBD | N/A | yes | stop/remove fixture | pending |
| TCC permissions | bundle id + visible toggle state | yes | no | restore chosen final state | pending |

Never print clipboard bodies, secret values, SSH private keys, proxy credentials, database
passwords, or subscription URLs into logs or the final report.

## 5. Authorized real external operations

The user authorized all categories below. Authorization does not remove target validation,
rollback, or user handoff for passwords, Touch ID, and macOS permission toggles.

### 5.1 Homebrew install, upgrade, and uninstall

Preferred target:

1. Create a namespaced temporary local tap/formula that installs a harmless test-owned executable.
2. Search and inspect it through Luma.
3. Test the install confirmation and cancel path.
4. Perform real install and verify the executable.
5. Publish a second local formula revision and perform real upgrade.
6. Perform real uninstall and verify removal.
7. Remove the temporary tap and verify existing user formulae/casks were unchanged.

Fallback:

- If the current Homebrew version cannot exercise a local formula safely, select a small,
  currently absent formula and record the exact target before installation.
- Never upgrade or uninstall an arbitrary existing user package.

### 5.2 System proxy

1. Resolve the active network service.
2. Snapshot HTTP, HTTPS, SOCKS, auto-proxy, exclusions, enabled flags, hostnames, and ports.
3. Start a controlled loopback test endpoint.
4. Exercise Luma status, check, mode/profile, enable, conflict, and disable paths.
5. Verify actual macOS proxy state after each successful action.
6. Restore the exact snapshot in a guaranteed cleanup step.
7. Compare final `networksetup`/system proxy state to the pre-state.

If macOS requests administrator authentication, the user completes it.

### 5.3 Apple Shortcuts

1. Enumerate and search real shortcuts without running them.
2. Verify duplicates, Unicode names, folders, exact-name View, missing command, and cancellation.
3. Inspect a candidate shortcut before execution.
4. Run only a shortcut proven to be harmless and not to transmit private data.
5. If no safe existing shortcut is available, create or request a clearly named temporary test
   shortcut rather than guessing.
6. Verify interactive terminal suspend/resume and exact exit feedback.

### 5.4 Keychain / Secrets

1. Use an isolated Luma support directory.
2. Create a unique account such as `luma-e2e-<run-id>` under service
   `com.luma.next.secrets`.
3. Exercise CLI stdin bootstrap without putting the value in argv or logs.
4. Enable the module, verify not-configured/locked/unlocked states, confirmation, copy, cancel,
   idle lock, teardown lock, and Keychain failure.
5. Delete only the exact test account and verify unrelated Keychain entries are unchanged.

The user handles any password or Touch ID prompt.

### 5.5 SSH and SFTP

Preferred target:

1. Start an isolated loopback SSH/SFTP server on an ephemeral port with a generated temporary key.
2. Create a temporary HOME and SSH config using concrete Host aliases and Include files.
3. Exercise list, search, preview, reload, favorite, recent, rename, copy alias, SSH, SFTP, failure,
   cancellation, and successful metadata recording.
4. Stop the exact server and remove generated keys/config.

Fallback:

- If an isolated server cannot run without changing macOS Remote Login, resolve an exact real SSH
  alias with the user before connecting.
- Authentication entry remains a user handoff.

### 5.6 PostgreSQL

Preferred target:

1. Use an existing local `postgres`/`psql` installation to initialize an ephemeral cluster under
   the test root.
2. Start it on an ephemeral loopback port with temporary credentials.
3. Add the portal, inspect metadata, open `psql`, test confirmation/cancel/failure, then remove
   portal metadata.
4. Stop the cluster and remove only the test cluster.

Fallback:

- If PostgreSQL is absent, either install a test dependency through the authorized Homebrew lane
  or obtain an exact external test database target.
- Do not connect to an arbitrary database discovered from user files.

### 5.7 macOS permissions

Screen Recording:

- [ ] Verify denied UI and module-local recovery guidance.
- [ ] User toggles/re-adds the final Luma app when requested.
- [ ] Verify granted window titles and OCR.
- [ ] Rebuild/re-sign and verify the grant persists for the stable designated requirement.

Accessibility:

- [ ] Verify denied focus/paste feedback.
- [ ] User enables Luma when requested.
- [ ] Verify real window focus and paste.
- [ ] Restart/rebuild and verify the intended final state.

Codex does not click security-sensitive permission toggles or enter system authentication.

### 5.8 Files and processes

- Use test-created files for rename, backup, reveal, open, and Trash.
- Use a test-created listener for Runtime SIGTERM and stale-PID validation.
- The broader authorization to delete a non-test file or terminate a non-test process is not a
  reason to choose a real user asset when a controlled fixture can prove the same feature.
- If a genuine non-test target is ever necessary, resolve its exact path/PID and capture its
  recoverability/state immediately before the action.

## 6. Cross-cutting native host and TUI checklist

### 6.1 Build, identity, and processes

- [ ] Cleanly stop the previous host/TUI before replacing the installed bundle.
- [ ] Build the Rust release binary and Swift release host.
- [ ] Build `/Users/diaoyuxuan/Applications/Luma.app` atomically.
- [ ] Verify plist executable, bundle identifier, minimum OS, LSUIElement, and single-instance key.
- [ ] Verify nested binaries and app bundle signatures.
- [ ] Verify stable designated requirement and note CDHash changes.
- [ ] Verify the app launches the bundled binary, not a stale target/debug or PATH binary.
- [ ] Verify exactly one host and one TUI after normal launch, `open -n`, and direct host launch.
- [ ] Verify no old bundle, duplicate process, or stale permission identity creates a false result.

### 6.2 Activation and lifecycle

- [ ] Cold launch.
- [ ] Cmd+Space with Spotlight disabled.
- [ ] Cmd+Space registration conflict with Spotlight enabled, including understandable feedback.
- [ ] Visible/active hotkey hides and restores the previous frontmost app.
- [ ] Hidden hotkey shows Luma and restores input focus.
- [ ] Visible/inactive hotkey brings Luma forward.
- [ ] Close button hides without killing the session.
- [ ] Minimize/restore preserves state.
- [ ] Repeated toggle does not duplicate sessions or accumulate state.
- [ ] TUI normal exit shows an honest exit notice.
- [ ] TUI signal exit is not misreported as a normal exit code.
- [ ] Next activation after TUI exit starts exactly one new session.
- [ ] Host quit terminates the child process group without leaving descendants.
- [ ] Memory-pressure scrollback policy preserves a usable session.

### 6.3 Input and keyboard

- [ ] First launch input focus.
- [ ] Hide/show input focus.
- [ ] Click-to-focus input.
- [ ] English typing and rapid continuous typing.
- [ ] Digits `0` through `9` in the prompt, including a leading `1`.
- [ ] Digits remain window shortcuts only while the Hub or `/win` list has focus.
- [ ] Simplified/Traditional Chinese input method composition and commit.
- [ ] Emoji, ZWJ emoji, combining marks, CJK, and mixed-width text.
- [ ] Paste multiline/Unicode text with control characters filtered safely.
- [ ] Backspace, Delete, word delete, cursor left/right, Home/End, Ctrl-A/E/U/W.
- [ ] Query history Ctrl-P/N.
- [ ] Enter, Esc, Tab, Shift-Tab, Ctrl-K, Ctrl-/, `?`.
- [ ] Arrow keys and Fn+Up/Down or PageUp/PageDown.
- [ ] ActionPicker digits and Wordbook review digits retain their scoped behavior.
- [ ] Terminal view, native window, overlay, and focus chain never swallow ordinary prompt text.

### 6.4 Rendering

- [ ] 80×24 compact layout.
- [ ] Narrow stacked preview.
- [ ] Wide side preview.
- [ ] Tall and short window sizes.
- [ ] Resize repeatedly while search/preview/loading is active.
- [ ] CJK width, emoji, combining marks, long titles/subtitles, long permission guidance.
- [ ] Loading-to-result replacement clears old glyphs.
- [ ] Overlay open/close clears underlying and overlay cells.
- [ ] Rapid selection changes never show a stale preview.
- [ ] Scroll markers, selection fill, badges, borders, and footer remain intact.
- [ ] No black blocks, transparent strips, blank panels, truncated concatenation, or preview residue.
- [ ] Terminal control filtering blocks unsafe OSC/APC while preserving valid UTF-8 across chunks.

### 6.5 Shared surfaces

- [ ] Empty Hub ordering: Windows, Continue, Modules.
- [ ] Hub scrolling, page movement, Enter, digits, and “more” row.
- [ ] Global search contributors and total/per-module caps.
- [ ] Result diversity and Recall ranking never beat clear semantic relevance.
- [ ] Informational rows never enter Recall or global results.
- [ ] `/commands [filter]`, completion, and `/help` agree with manifests.
- [ ] `/settings` module enable/disable and every personal setting persist through CAS.
- [ ] Disabled modules do not warm up or appear on Hub.
- [ ] Overlays restore or discard the prompt according to their contract.
- [ ] Confirm and cancel overlays never leak the underlying keypress.
- [ ] Status tones distinguish success, permission, unavailable, not configured, cancelled, and error.
- [ ] Search cancellation and late events cannot overwrite a newer query.

## 7. CLI checklist

- [ ] `luma tui` default and explicit invocation, including `--initial-query <TEXT>`.
- [ ] `luma query` text and JSON output, redaction, exit codes, and invalid config.
- [ ] `luma action run` safe, confirmation-required, cancelled, failed, and successful actions,
      including `--query`, optional `--result-id`, `--action-id`, `--confirmation`, `--json`, and
      `--redact`.
- [ ] `luma modules list` inventory, enable state, and JSON purity; verify its intentionally
      limited id/enabled/display-name schema rather than using it to infer commands or triggers.
- [ ] `luma config get` and `luma config set`: concurrent CAS, invalid values, sticky module keys,
      and every set flag: `--records-root`, repeatable `--projects-root`,
      `--enable-module`, `--disable-module`, `--clipboard-retention-days`,
      `--secrets-idle-lock-secs`, `--hub-windows-max`, `--proxy-controller-unix-socket`,
      `--proxy-controller-address`, `--proxy-controller-secret-account`,
      `--proxy-network-service`, repeatable `--import-project` and `--remove-project`,
      `--expected-version`, and `--json`.
- [ ] `luma migrate dry-run`, `clipboard-fixture`, `clipboard`, `list`, and `rollback`;
      clipboard writes use `--commit` (there is no generic `migrate apply`), and source files
      remain immutable.
- [ ] `luma secrets set <account>` stdin-only value handling and label sidecar.
- [ ] `luma wordbook import-wordpet --from <PATH>` dry-run by default, `--commit`, and `--json`;
      `luma wordbook backup`; verify that no nonexistent wordbook query or `--apply` form is
      documented or accepted.
- [ ] `luma record status`, `browse`, `add`, `rate`, `note`, `remove`, `import`,
      `import-status`, and `backup`, including `browse --category`, add `--rating`/`--note`, rate
      `--clear`, remove `--yes`, import `--root`/`--apply`, and each supported `--json`; record
      import writes with `--apply`, and there is no `luma record rollback` subcommand.
- [ ] For a committed Records import, exercise rollback only through
      `luma migrate rollback --migration-id <ID>` and verify unrelated files and other migration
      artifacts remain untouched.
- [ ] `luma cmd list`, `show`, `run`, and `copy`, exact argv, missing recipe, confirmation, JSON,
      and exit propagation.
- [ ] `luma ssh` list/connect/sftp/favorite/unfavorite/rename and invalid alias.
- [ ] No retired bare-trigger, colon command, notes module, or doctor entry point is accepted.

## 8. Module-by-module checklist

### 8.1 Apps — `/app`, `/apps`

- [ ] Warmup, cached list, exact trigger, alias, global fuzzy search, and session MRU.
- [ ] Compact fuzzy matches win over loose noise.
- [ ] Unicode, spaces, nested apps, symlinked bundles, duplicate names, and missing executables.
- [ ] Launch, reveal in Finder, and copy path.
- [ ] Stale/missing cached path fails honestly.
- [ ] Cancellation and warmup failure remain visible.
- [ ] Exercise every Apps row whose resolved action is `noop`; it has no side effect or Recall
      entry.

### 8.2 Calculator — `/calc`, `/calculate`

- [ ] Arithmetic precedence, parentheses, associativity, unary values, percent, and invalid syntax.
- [ ] Length/depth/finite-number limits and division/error cases.
- [ ] Every supported linear unit family and temperature offsets.
- [ ] Integer bases and exact signed integer boundaries.
- [ ] Unix timestamp conversion and date offsets.
- [ ] Strict complete-expression global detection rejects ordinary prose.
- [ ] Copy result and copy equation; pasteboard failure/cancel never reports success.
- [ ] Exercise `copy_decimal` and `copy_hex` for applicable integer/base results, including
      unavailable-action boundaries.
- [ ] Exercise every Calculator row whose resolved action is `noop`; it has no side effect or
      Recall entry.

### 8.3 Downloads Inbox — `/dl`, `/downloads`

- [ ] Default, recent, large, old, type, and text-filtered views.
- [ ] Direct children only, deterministic sorting, scan cap, and cancellation.
- [ ] Unicode, spaces, control characters, invalid UTF-8 filenames, directories, and symlinks.
- [ ] Open, reveal, and copy path.
- [ ] Rename validation, collision, stale identity, extension-change confirmation, cancel, and success.
- [ ] Finder Trash confirmation, cancel, stale identity, success, and recoverability.
- [ ] Missing Downloads directory and adapter failures remain distinct.
- [ ] Exercise every Downloads row whose resolved action is `noop`; it has no side effect or
      Recall entry.

### 8.4 Packages — `/pkg`, `/packages`, `/brew`

- [ ] Installed, outdated, formulae, casks, search, and info views.
- [ ] Missing Homebrew, timeout, nonzero exit, malformed JSON, oversized output, and cancellation.
- [ ] Exact formula/cask identity and stale result rejection.
- [ ] Install confirmation/cancel/real install.
- [ ] Upgrade confirmation/cancel/real upgrade.
- [ ] Uninstall confirmation/cancel/real uninstall.
- [ ] Interactive terminal suspend/resume and exact argv without shell interpolation.
- [ ] Exercise `show_info`, `copy_name`, and `copy_homepage` on applicable formula/cask results;
      missing homepage and stale identities fail honestly.
- [ ] Cleanup temporary tap/formula and verify existing packages unchanged.
- [ ] Exercise every Packages row whose resolved action is `noop`; it has no side effect or Recall
      entry.

### 8.5 Apple Shortcuts — `/sc`, `/shortcut`, `/shortcuts`

- [ ] Default list, search, and custom-folder views.
- [ ] Discover and execute the exact `/sc folders` and `/sc folder <exact-name>` command forms.
- [ ] Spaces, Unicode, empty output, duplicate names, and reordering-stable identities.
- [ ] No enumeration during warmup.
- [ ] Exact-name View.
- [ ] Exact-name interactive Run.
- [ ] Exercise `open_folder` and `copy_name` actions.
- [ ] Missing executable, timeout, nonzero exit, cancellation, and stale duplicate.
- [ ] No implicit input, URL fallback, or captured shortcut output is introduced.
- [ ] Exercise every Shortcuts row whose resolved action is `noop`; it has no side effect or
      Recall entry.

### 8.6 Shell Recall — `/hist`, `/history`

- [ ] Plain zsh history, extended format, multiline commands, Unicode, and invalid UTF-8.
- [ ] Discover and execute `/hist recent` separately from filtered search.
- [ ] Tail-only read, entry/byte caps, deduplication, and stable identities.
- [ ] Credential/token/PEM/NUL/oversized command suppression.
- [ ] Search, no-match, missing history, unreadable history, and cancellation.
- [ ] Primary copy action only; no execute action.
- [ ] Command text never enters Recall or logs.
- [ ] Exercise every Shell Recall row whose resolved action is `noop`; it has no side effect or
      Recall entry.

### 8.7 Renewals — `/renew`, `/renewals`

- [ ] Upcoming, due, 30-day, search, empty, and capacity states.
- [ ] Add and edit every supported recurrence.
- [ ] Integer minor units and explicit currency precision.
- [ ] Month-end, leap-year, quarterly, custom, and once anchors.
- [ ] Paid idempotency and advancement from scheduled date.
- [ ] Cancel and delete confirmation, cancellation, stale identity, and success.
- [ ] Completed one-time renewal restrictions.
- [ ] Backup creation, naming, atomicity, and SQLite readability.
- [ ] Store failure and conflict feedback.
- [ ] Exercise every Renewals row whose resolved action is `noop`; it has no side effect or Recall
      entry.

### 8.8 Database Portals — `/db`, `/database`, `/databases`

- [ ] Default-off discovery and Settings enable/disable.
- [ ] Not configured and configured-empty states.
- [ ] Canonical SQLite add, duplicate/path traversal/symlink/oversize rejection.
- [ ] SQLite open, reveal, tables, indexes, bounded DDL, and read-only behavior.
- [ ] PostgreSQL portal fields without passwords or DSNs.
- [ ] PostgreSQL open through exact `psql` argv and existing libpq authentication.
- [ ] Production-open confirmation, cancel, failure, and last-open bookkeeping.
- [ ] Metadata-only remove confirmation and identity revalidation.
- [ ] Backup atomicity and readability.
- [ ] No discovery, SQL editor, connection-test, or credential leakage is introduced.
- [ ] Exercise every Database Portals row whose resolved action is `noop`; it has no side effect
      or Recall entry.

### 8.9 Screen OCR — `/ocr`

- [ ] No-argument contract and rejection of unexpected arguments.
- [ ] Screen Recording denied guidance.
- [ ] User-cancelled region selection.
- [ ] Empty capture and screencapture failure.
- [ ] Simplified Chinese, Traditional Chinese, English, mixed text, and emoji-adjacent content.
- [ ] Plain UTF-8 result bounded to 256 KiB.
- [ ] Execute `capture_copy` and verify recognized text is copied to the clipboard only; OCR does
      not auto-paste into the previously focused app and does not require Accessibility.
- [ ] Verify the copied text can be pasted manually by the user without entering Recall or logs.
- [ ] Clipboard failure and cancellation.
- [ ] Private temporary capture deleted on every return path.
- [ ] Exercise every Screen OCR row whose resolved action is `noop`; it has no side effect or
      Recall entry.

### 8.10 Windows — `/win`, `/window`, `/windows`

- [ ] Hub projection, targeted list, aliases, search by application/title, and configured cap.
- [ ] Real titles with Screen Recording granted.
- [ ] Redacted-title permission guidance with Screen Recording denied.
- [ ] Genuine untitled windows are not misdiagnosed.
- [ ] CJK/emoji/long window titles.
- [ ] Prompt digits remain text.
- [ ] Hub and `/win` list-focus digits map to the visible 1–9 windows.
- [ ] Enter and digit focus with Accessibility denied.
- [ ] Enter and digit focus with Accessibility granted.
- [ ] Stale window identity and disappearing app/window.
- [ ] Foreground handoff and Luma hide/show behavior.
- [ ] Exercise the `refresh` action after permission changes and window-list churn.
- [ ] Exercise every Windows row whose resolved action is `noop`; it has no focus change or Recall
      entry.

### 8.11 Git — `/git`

- [ ] Imported-project-only discovery and direct unimported path denial.
- [ ] Dashboard ordering for conflict, dirty, ahead, behind, and clean repositories.
- [ ] `/git repo`, branches, log, and commit surfaces.
- [ ] Staged, unstaged, untracked, renamed, deleted, binary, and conflicted files.
- [ ] Stage and unstage exact paths.
- [ ] Exercise `stage_all` and `unstage_all` with mixed file states and identity revalidation.
- [ ] Discard only tracked non-conflicted unstaged content with confirmation.
- [ ] Discard cancel and preservation of index/untracked files.
- [ ] Clean repositories hide irrelevant actions.
- [ ] Branch switching only while clean.
- [ ] Commit validation, exact message handling, success, and failure.
- [ ] Exercise `copy_path`, `copy_branch`, and `copy_sha` only on applicable result kinds.
- [ ] Exercise result/UI routes `open_workbench`, `open_branches`, and `open_log`, verifying exact
      project identity and prompt restoration.
- [ ] Path traversal, NUL, absolute path, timeout, and cancellation.
- [ ] No remote/fetch/push/pull/clone/rebase/reset/clean behavior appears.
- [ ] Exercise every Git row whose resolved action is `noop`; it has no side effect or Recall
      entry.

### 8.12 Runtime — `/run`, `/ports`

- [ ] On-demand listener list, filters, missing `lsof`, timeout, and cancellation.
- [ ] IPv4, IPv6, wildcard, loopback, process, user, cwd, and project association.
- [ ] Copy port/address/PID fields.
- [ ] Exercise `copy_process` as a distinct action and verify copied display text matches the
      freshly listed process identity.
- [ ] Exercise `refresh` and the associated-project `open_project` primary route.
- [ ] Protected names, other-user processes, and unavailable identity.
- [ ] Terminate confirmation and cancel.
- [ ] Fresh re-list detects stale PID/reused identity.
- [ ] Real SIGTERM succeeds only against the test-owned listener.
- [ ] No background monitor or SIGKILL.
- [ ] Exercise every Runtime row whose resolved action is `noop`; it has no side effect or Recall
      entry.

### 8.13 Proxy — `/proxy`, `/px`

- [ ] Overview, status, groups, group drilldown, nodes, mode, and on-demand check.
- [ ] Discover and execute `/proxy global`, `/proxy rule`, and `/proxy profile refresh`
      explicitly.
- [ ] Missing controller, unavailable controller, timeout, malformed/chunked HTTP, and cancellation.
- [ ] Controller secrets/endpoints never appear in result IDs, UI, logs, or outcomes.
- [ ] Luma profile import from safe local file and subscription.
- [ ] YAML, base64, and common node URI parsing.
- [ ] Dangerous runtime/listener/script fields rejected before persistence.
- [ ] Profile list, use, refresh, delete, rollback, and Keychain URL handling.
- [ ] Exercise node `select_proxy`, provider `refresh`, and `copy_proxy_address` actions with exact
      group/provider identity.
- [ ] Clash Verge profile read-only behavior unless Luma-owned.
- [ ] Real HTTP/SOCKS system proxy enable/check/disable.
- [ ] HTTPS state remains read-only and honest.
- [ ] Pre-existing complex proxy state conflict and exact final restoration.
- [ ] Exercise every Proxy row whose resolved action is `noop`; it has no proxy mutation or Recall
      entry.

### 8.14 Clipboard — `/clip`, `/cb`

- [ ] Capture start, teardown, missing store, and unavailable store.
- [ ] Plain text, Unicode, multiline, duplicates, and entry-size boundary.
- [ ] Concealed/transient/autogenerated password-manager markers are skipped.
- [ ] History search, copy, paste, pin, unpin, and stale identity.
- [ ] Exercise per-entry `delete`, including confirmation/cancel if offered, stale identity, and
      preservation of every other entry.
- [ ] Paste with Accessibility denied and granted.
- [ ] Clear confirmation/cancel/success.
- [ ] Pause, bounded duration, automatic resume, explicit resume, and status.
- [ ] 500 unpinned eviction, 100 pinned hard cap, and pinned preservation.
- [ ] Clipboard and search text privacy in Recall/logs.
- [ ] Restore the original clipboard without disclosing its body.
- [ ] Exercise every Clipboard row whose resolved action is `noop`; it has no side effect or
      Recall entry.

### 8.15 Quicklinks — `/ql`, `/quicklinks`

- [ ] Empty/list/search states and both aliases.
- [ ] Exercise onboarding `seed_add` and verify it opens/prefills only the intended add surface.
- [ ] Add URL validation, mixed case, spaces, Unicode, and hard cap.
- [ ] Duplicate overwrite confirmation and cancel.
- [ ] Open and copy URL.
- [ ] Delete confirmation/cancel/success.
- [ ] Stale identity and unavailable store.
- [ ] Backup atomicity and SQLite readability.
- [ ] Exercise every Quicklinks row whose resolved action is `noop`; it has no side effect or
      Recall entry.

### 8.16 Snippets — `/s`, `/snip`

- [ ] Empty/list/search states and both aliases.
- [ ] Exercise onboarding `seed_add` and verify it opens/prefills only the intended add surface.
- [ ] Add, duplicate overwrite confirmation, cancel, and hard cap.
- [ ] Add-from-clipboard with multiline/Unicode body.
- [ ] Copy without Accessibility.
- [ ] Paste with Accessibility denied and granted.
- [ ] Delete confirmation/cancel/success.
- [ ] Stale identity, clipboard failure, and unavailable store.
- [ ] Backup atomicity and SQLite readability.
- [ ] Snippet bodies never enter Recall.
- [ ] Exercise every Snippets row whose resolved action is `noop`; it has no side effect or Recall
      entry.

### 8.17 Wordbook — `/wb`, `/wordbook`, `/words`

- [ ] Today, due, new, wrong, search, empty, and import onboarding.
- [ ] Discover and exercise `/wb add` and `/wb status` as separate command surfaces.
- [ ] Clipboard and CSV import, duplicate handling, oversized/read failure, and cancellation.
- [ ] Daily goal read/update and review queue fill behavior.
- [ ] `/wb review`, `review due`, `review new`, and `review wrong`.
- [ ] Enter/Space reveal.
- [ ] `1` known, `2` fuzzy, `3` unknown.
- [ ] Grade blocked before reveal.
- [ ] `m` mastered confirmation/cancel/success.
- [ ] `s` skip and Esc exit/cancel.
- [ ] Progress, completion summary, counters, and speech paths.
- [ ] Exercise onboarding `seed_add`, `unmaster`, `speak`, `speak_example`, `copy_term`, and
      `delete`, including applicability, confirmation/cancel where offered, stale identity, and
      adapter failure.
- [ ] Rapid grade/cancel and stale async update.
- [ ] Backup atomicity and SQLite readability.
- [ ] CJK, emoji, combining text, long titles/subtitles, and resize during loading.
- [ ] Exercise every Wordbook row whose resolved action is `noop`; it has no side effect or Recall
      entry.

### 8.18 Records — `/rec`, `/record`

- [ ] Search, browse, recent, unrated, top, category, and empty onboarding.
- [ ] Discover and execute `/rec import` and `/rec status`; classify the hidden parser alias
      `/rec ls [category]` through the command-discovery parity item in section 2.1.
- [ ] Add every supported record kind and metadata.
- [ ] Exercise `open` (View/preview), `rate`, `note` (Edit note), and `remove`; there is no generic
      record edit action.
- [ ] Exercise onboarding `seed_config` without inventing a centralized setup/doctor surface.
- [ ] Validation, cancel, stale identity, and store conflict.
- [ ] CLI import dry-run, apply, idempotency, and ledger.
- [ ] Imported source Markdown remains read-only.
- [ ] Changed source does not overwrite newer database edits.
- [ ] `luma migrate rollback --migration-id <ID>` restores only the Records migration artifact;
      do not document or test a nonexistent `luma record rollback`.
- [ ] Backup atomicity and SQLite readability.
- [ ] CJK/emoji/long titles, notes, and categories.
- [ ] Exercise every Records row whose resolved action is `noop`; it has no side effect or Recall
      entry.

### 8.19 Projects — `/p`, `/proj`, `/project`

- [ ] Empty state, all aliases, search, Recall ranking, and ambiguous name handling.
- [ ] Add/import canonical directory.
- [ ] Resolve the parser/manifest discovery drift for `/proj import PATH`; exercise both accepted
      add/import forms only after palette/help/completion agree.
- [ ] Duplicate, missing, file, symlink, traversal, relative, Unicode, and space paths.
- [ ] Browse directory ordering, bounds, `..`, symlink escape, and cancellation.
- [ ] Remove confirmation/cancel/success changes config only.
- [ ] Verify project directory is never deleted.
- [ ] `/proj show` exact name/path resolution.
- [ ] Aggregated Continue, Git, Runtime, Recipes, files, Finder, editor, and terminal rows.
- [ ] Each aggregation failure remains local to its row.
- [ ] Exercise result routes `seed_config`, `open_workbench`, `continue_project`, `open_git`,
      `open_runtime`, `open_recipes`, and `open_files` with exact imported-project identity.
- [ ] Open Finder, available editor, and project-rooted terminal.
- [ ] Settings CAS conflict and restart persistence.
- [ ] Exercise every Projects row whose resolved action is `noop`; it has no side effect or Recall
      entry.

### 8.20 Command Recipes — `/cmd`, `/recipe`, `/recipes`

- [ ] Default current-directory view, all aliases, `/cmd all`, filter, and `/cmd project`.
- [ ] Built-in and user TOML recipes.
- [ ] Applicable/inapplicable ordering and missing programs.
- [ ] `.git` directory, worktree file, executable symlink, and PATH changes.
- [ ] Exact ordered program/args/environment with no shell interpolation.
- [ ] Run, copy, favorite, and missing recipe.
- [ ] Exercise `preview`, `show_variant`, `unfavorite`, and `open_config`, including applicability,
      missing-config, and stale recipe identity.
- [ ] Confirmation by risk, cancel, nonzero exit, signal exit, and success.
- [ ] TUI suspend/resume in the native host.
- [ ] Project identity revalidation and cancellation.
- [ ] Exercise every Command Recipes row whose resolved action is `noop`; it has no side effect or
      Recall entry.

### 8.21 SSH — `/ssh`

- [ ] Missing config, unreadable config, parser failure, and unavailable `ssh`.
- [ ] Concrete aliases, wildcard exclusion, Include files, depth cap, spaces, and Unicode.
- [ ] Automatic refresh and explicit reload.
- [ ] Search, favorite, recent, rename with multiword display name, and 1000-row cap.
- [ ] Exercise `unfavorite`, `delete_metadata`, and `reload_config`, including stale identity and
      config reorder.
- [ ] Stable metadata across config reordering.
- [ ] Preview fields without private-key content.
- [ ] Copy alias.
- [ ] SSH and SFTP exact argv with end-of-options protection.
- [ ] Unknown/stale alias rejection.
- [ ] Interactive success invokes the internal post-session `record_connection` path; cancel,
      signal, and nonzero exit do not record metadata.
- [ ] TUI suspend/resume and authentication handoff.
- [ ] Exercise every SSH row whose resolved action is `noop`; it has no side effect or Recall
      entry.

### 8.22 Timers — `/tm`, `/timer`, `/timers`

- [ ] Empty/list/search states and every alias.
- [ ] Stopwatch create/start and named variants.
- [ ] Countdown shorthand, bounded duration, Pomodoro default/custom duration.
- [ ] Classify and exercise parser-only aliases `/tm start [name]`, `/tm stopwatch [name]`,
      `/tm pomodoro ...`, `/tm cd ...`, and `/tm countdown ...` after resolving manifest/palette
      discovery parity.
- [ ] Start, pause, resume, reset, and delete confirmation/cancel/success.
- [ ] Running/paused Hub Continue natural actions.
- [ ] Wall-clock/monotonic deadline behavior and store conflicts.
- [ ] Completion once-only state transition and speech alert.
- [ ] Teardown cancellation prevents late completion/alert.
- [ ] Graceful app quit pauses running timers.
- [ ] Restart persistence and 256-row capacity.
- [ ] Exercise every Timers row whose resolved action is `noop`; it has no side effect or Recall
      entry.

### 8.23 Secrets — `/sec`, `/secret`, `/secrets`

- [ ] Default-off Settings behavior.
- [ ] No labels: not-configured bootstrap guidance.
- [ ] Exercise onboarding `seed_config` and verify it points only to the documented local
      bootstrap/configuration path.
- [ ] CLI stdin-only bootstrap and sidecar label update.
- [ ] Keychain/sidecar unavailable states.
- [ ] Search exposes labels only, never values.
- [ ] Locked vault, unlock flow, and cancellation.
- [ ] Exercise explicit manual `lock` in addition to idle and teardown locking.
- [ ] Copy confirmation/cancel/success.
- [ ] Idle lock default, custom value, disabled value, teardown lock, and restart lock.
- [ ] Exact test Keychain account cleanup.
- [ ] Exercise every Secrets row whose resolved action is `noop`; it has no side effect or Recall
      entry.

## 9. Cross-module and persistence checklist

- [ ] Global contributors: Apps, Calculator, Windows, Projects, Recipes, SSH, Clipboard, Snippets,
      Quicklinks, and Git.
- [ ] Records and Wordbook remain targeted-only.
- [ ] Per-module 12-result cap and total 60-result cap.
- [ ] Recall stores only bounded safe metadata after successful actions.
- [ ] Failed/cancelled actions never enter Recall.
- [ ] Clipboard/snippet bodies, SSH configuration, proxy endpoints, calculator expressions, OCR
      text, and raw search text never enter Recall.
- [ ] Hub Continue prioritizes live/paused Timers, then bounded Recall.
- [ ] Direct Git/Runtime payload is revalidated rather than blindly continued.
- [ ] Settings survive TUI/app restart and concurrent CAS conflicts.
- [ ] Database/file backups are created atomically and can be opened/read.
- [ ] Logs rotate at the documented limit and never expose sensitive values.
- [ ] Rebuild/re-sign keeps intended TCC identity.
- [ ] No user database, project, file, Keychain entry, proxy state, or process is changed outside
      the recorded targets.

## 10. Defect loop

For every reproducible issue:

1. Record exact build, state, keystrokes/command, expected result, and actual result.
2. Identify whether the fault belongs to host, PTY/filter, TUI reducer/render, Engine, module,
   adapter, persistence, permissions, or external dependency.
3. Make the smallest reliable change with `apply_patch`.
4. Add a targeted automated test that fails before the fix.
5. Run formatting and the affected crate/package tests.
6. Rebuild the final installed app.
7. Repeat the exact real UI reproduction.
8. Rerun the affected module's complete checklist.
9. At batch boundaries, rerun the full verification set.
10. Commit only coherent fixes; never commit probes, fixture secrets, local logs, or launch flags.

Defect evidence template:

| Field | Value |
| --- | --- |
| ID | `UX-XXX` |
| Module/surface | TBD |
| Reproduction | TBD |
| Expected | TBD |
| Actual | TBD |
| Root cause | TBD |
| Fix | TBD |
| Automated test | TBD |
| Installed-app UI retest | TBD |
| Commit | TBD |

## 11. Verification commands

Run at every batch boundary and once more after the final source change:

```bash
cd /Users/diaoyuxuan/Luma/rust
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p luma --test cli_blackbox
./scripts/check_architecture.sh
swift test --package-path native/luma-workbench
swift build --package-path native/luma-workbench -c release
./scripts/build_workbench_app.sh /Users/diaoyuxuan/Applications/Luma.app
codesign --verify --deep --strict /Users/diaoyuxuan/Applications/Luma.app
git diff --check
```

After rebuilding:

- [ ] Verify installed app identity and exact bundled executable.
- [ ] Verify exactly one host and one TUI.
- [ ] Verify Screen Recording behavior.
- [ ] Run the native input/lifecycle smoke set.
- [ ] Run Hub, global search, `/commands`, `/settings`, `/win`, and `/wb` smoke set.

## 12. Cleanup and completion criteria

Cleanup:

- [ ] Stop every test-owned server/listener/PTY.
- [ ] Restore system proxy exactly.
- [ ] Restore clipboard without printing it.
- [ ] Remove only namespaced test Keychain accounts.
- [ ] Remove temporary Homebrew formula/tap and verify existing packages.
- [ ] Remove test projects, databases, histories, SSH config/keys, imports, and logs.
- [ ] Confirm no test process, temporary helper bundle, probe, or fixture remains.
- [ ] Confirm the user's LumaNext data and non-test files were not deleted or overwritten.
- [ ] Confirm the desired final TCC permission state.
- [ ] Confirm git status contains only intended source/doc changes.

The review is complete only when:

- [ ] The machine-enumerated composition/manifest/CLI/action inventory has no unexplained
      difference from the 23-module/97-`CommandSpec` baseline or the human checklist.
- [ ] Every declared module command, accepted parser alias, result action, `ui_intent`, internal
      post-session action, and module-specific `noop` row has a recorded status.
- [ ] Parser, `/commands`, `/help`, completion, module documentation, and tests agree on every
      accepted interactive command.
- [ ] Every production module has been exercised in isolated real-process and applicable installed
      app UI lanes.
- [ ] All authorized external effect paths are either real `PASS`/`FIXED` or precisely `BLOCKED`
      by an unavailable target/authentication requirement.
- [ ] No reproducible core usability defect remains.
- [ ] Full Rust, Swift, architecture, build, and signing verification passes after the final fix.
- [ ] The final installed app has been manually exercised after the final rebuild.
- [ ] All test-created assets are cleaned and all snapshotted system state is restored.
- [ ] Commits are semantic and contain no probes, secrets, local logs, or temporary launch behavior.
- [ ] Final report lists fixes, evidence, complete coverage status, and any remaining user actions.
