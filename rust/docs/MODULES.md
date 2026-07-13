# Modules

Living status for the Rust CLI/TUI. Prefer honest `unavailable` / `permission_required` / `not_configured` over empty results or silent stubs that look finished.

## Core

| Area | Status | Notes |
| --- | --- | --- |
| Doctor / diagnostics | Available | `luma doctor --json`, TUI `:doctor`; redaction; corrupt-config visibility |
| Config | Available | Versioned settings; CAS; `luma config`; TUI `:settings` |
| Module registry | Available | Manifest, enable/disable cancels in-flight work |

## Modules

| Module | Triggers | Status | Notes |
| --- | --- | --- | --- |
| Apps | `app` / `apps` | Available | Launch/focus/reveal/quit; warm index; no per-keystroke disk scan |
| Clipboard | `clip` / `cb` | Available | History, pin/delete, privacy filters; paste needs Accessibility |
| Notes | `n` / `note` / `notes` | Available | Root containment; create/open/daily/capture; FSEvents + poll |
| Quicklinks | `ql` | Available | http/https/mailto; templates; confirm destructive edits |
| Snippets | `s` / `snip` | Available | Copy always; paste/insert needs AX; search path must not await AX |
| Translate | `tr` / `translate` | Unavailable / gated | Port exists; live Translation needs signed host |
| Todo | `t` / `todo` | Gated | EventKit; permission row when denied/undetermined |
| Projects | `proj` | Available (shallow→deep) | Workbench-style commands |
| Kill | `kill` | Available | Process list + terminate with confirm |
| Media | `media` | Shallow | Expand only with proven storage/IO |
| Window layouts | — | Stub / permission | AX-gated |
| Menu bar search | — | Stub / permission | AX-gated |
| Browser tabs | — | Stub / permission | Automation-gated |
| Secrets | — | Labels only | Default off; Keychain namespace `com.luma.next.secrets`; never search values |
| Wordbook | `word` | Unavailable | Explicit unavailable until storage lands |
| Fake | — | Default off | Test/demo module |

## Product rules

- UI is terminal CLI/TUI only (no native GUI or web product shell).
- Platform calls stay behind ports in `luma-platform-macos`.
- Tests/soak must not steal focus: no `open`, osascript, AX paste, or system clipboard mutation.
- Destructive actions require confirm; cancel must be real.
