# ADR-0003: macOS permission spike results (CLI probe)

- Status: Accepted (partial — CLI probe only; no TCC prompts forced)
- Date: 2026-07-13

## Context

Evidence for Pasteboard, Accessibility, Automation, EventKit, Translation, and Keychain from a Terminal-launched Rust binary. Interactive TCC prompts were not forced.

## Evidence

| Capability | Probe | Result |
| --- | --- | --- |
| Pasteboard | `pbcopy` / `pbpaste` via `MacPasteboard` | Works from Terminal identity; denied vs success distinguishable |
| Accessibility | `AXIsProcessTrusted` + Cmd+V CGEvent synthesis | Trust query works; paste reports `PermissionRequired` when untrusted — never success |
| Automation | `MacAutomation` | NotDetermined / Unavailable without signed host + user consent |
| EventKit | `MacEventKit` | CLI cannot request Reminders auth without app bundle + usage string → NotDetermined + guidance |
| Translation | `MacTranslator` | Unavailable from pure Rust CLI (needs app-host bridge) |
| Keychain | `security` CLI, service `com.luma.next.secrets` | Labels/values via explicit copy; search never includes values |

## Decisions

1. Product UI stays CLI/TUI; thin signed app host is optional later for EventKit/Automation/Translation prompts.
2. Modules use ports and prefer permission-denied / unavailable taxonomy over empty results.
3. Secrets remain default-off; Keychain namespace is LumaNext-only (`com.luma.next.secrets`).
4. Optional migrate importers are dry-run by default and never write the source tree.

## Consequences

- Todo / Browser Tabs / Translate remain permission- or host-gated until a signed host exists.
- Clipboard / Snippets paste works when Accessibility is granted to the Terminal/`luma` process.
- Notes watch uses `notify` (FSEvents on macOS) with polling fallback.
