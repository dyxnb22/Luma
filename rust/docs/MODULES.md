# Modules

Living status for the Rust CLI/TUI. Prefer honest `unavailable` / `permission_required` / `not_configured` over empty results or silent stubs that look finished.

## Core

| Area | Status | Notes |
| --- | --- | --- |
| Doctor / diagnostics | Available | `luma doctor --json`, TUI `:doctor`; redaction; corrupt-config visibility |
| Config | Partial | Versioned settings; CAS; `luma config get/set`. TUI settings route not implemented (use CLI) |
| Module registry | Available | Manifest + enable/disable; disable cancels module-scoped in-flight search/action and teardowns; session warmup covers enabled modules only |

## Modules

| Module | Triggers | Status | Notes | Default |
| --- | --- | --- | --- | --- |
| Apps | `app` / `apps` | Available | Launch / reveal / copy path; warm memory index; exact trigger lists cached apps. No focus/quit yet | on |
| Clipboard | `clip` / `cb` | Available | History, pin/delete, privacy filters; paste needs Accessibility | on |
| Notes | `n` / `note` / `notes` | Partial | Root containment; search/open/`n new`; FSEvents + poll. No daily/capture yet | on |
| Quicklinks | `ql` | Available | http/https/mailto; new add is Safe; overwrite + delete require confirm | on |
| Snippets | `s` / `snip` | Available | Copy always; paste/insert needs AX; search path must not await AX | on |
| Translate | `tr` / `translate` | Unavailable / gated | Port exists; live Translation needs signed host | on (gated) |
| Todo | `t` / `todo` | Gated | EventKit; permission row when denied/undetermined | on (gated) |
| Projects | `proj` | Shallow | One-level directory scan + open only (not workbench shallow→deep) | **off** |
| Kill | `kill` | Available | Process list; quit/force require confirm (Engine + TUI confirm route) | **off** |
| Media | `media` | Shallow | Expand only with proven storage/IO | **off** |
| Window layouts | — | Stub / permission | AX-gated | off |
| Menu bar search | — | Stub / permission | AX-gated | off |
| Browser tabs | — | Stub / permission | Automation-gated | off |
| Secrets | — | Labels only | Default off; Keychain namespace `com.luma.next.secrets`; never search values | **off** |
| Wordbook | `word` | Unavailable | Explicit unavailable until storage lands | off |
| Fake | — | Default off | Test/demo module | **off** |

## Product rules

- UI is terminal CLI/TUI only (no native GUI or web product shell).
- `bins/luma` is the sole composition root; `luma-tui` depends on application ports only.
- Platform calls stay behind ports in `luma-platform-macos`.
- Tests/soak must not steal focus: no `open`, osascript, AX paste, or system clipboard mutation.
- Destructive / Confirm actions require confirm; cancel must be real (search + in-flight operations).
- Operation cancel/disable awaits the operation task after signalling the token; modules race awaitable I/O against the token. Side effects already committed (e.g. kill signal sent) are not rolled back.
- Protocol `ActionOutcomeDto::Failed` carries structured `FailureKind` (not Debug strings).
- `bins/luma` injects all production adapters/stores; module `new()` constructors that hide Mac/store wiring are removed for Quicklinks/Snippets/Notes/Todo/Kill/Secrets.
