# ADR-0003: macOS permission spike results (CLI probe)

- Status: Accepted (partial — CLI probe only; no TCC prompts forced)
- Date: 2026-07-13

## Context

What a Terminal-launched Rust binary can and cannot do for Pasteboard, Accessibility,
and Keychain. Interactive TCC prompts were not forced.

## Evidence

| Capability | Probe | Result |
| --- | --- | --- |
| Pasteboard | `pbcopy` / `pbpaste` via `MacPasteboard` | Works from Terminal identity; denied vs success distinguishable |
| Accessibility | `AXIsProcessTrusted` + Cmd+V CGEvent synthesis | Trust query works; paste reports `PermissionRequired` when untrusted — never success |
| Keychain | `security` CLI, service `com.luma.next.secrets` | Labels/values via explicit copy; search never includes values |
| Windows list | `CGWindowListCopyWindowInfo` (`windows.list`) | Lists layer-0 windows; titles may be empty without Screen Recording |
| Windows focus | AX raise + frontmost (`ax.trusted`) | Requires Accessibility; tests never call real focus |

## Decisions

1. Product UI stays CLI/TUI for personal use; no signed app host in scope.
2. Modules use ports and prefer permission-denied / unavailable taxonomy over empty results.
3. Secrets remain default-off; Keychain namespace is LumaNext-only (`com.luma.next.secrets`).
4. Optional migrate importers are dry-run by default and never write the source tree.

## Consequences

- Clipboard / Snippets paste works when Accessibility is granted to the Terminal/`luma` process.
- Notes watch uses `notify` (FSEvents on macOS) with polling fallback.
