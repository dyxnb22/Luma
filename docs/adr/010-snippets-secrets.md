# ADR-010: Snippets + Secrets in v0.1

## Status

Accepted. Supersedes the "Secrets as first-class UX" exclusion in `docs/ROADMAP.md` v0.1 row and the "First-class password manager" line in `docs/NON_GOALS.md` for the developer-credentials slice.

> **Historical / amended by ADR-023 and ADR-032.** Do not implement dashboard/card instructions in this ADR. Route C uses prefix triggers and same-panel detail only.

Date: 2026-06-22

## Context

Two workflows keep heavyweight apps open:

1. **Cheatsheet lookup** — git incantations, docker one-liners, boilerplate text. Previously satisfied by TextExpander, Espanso, Alfred Snippets, or a scratch buffer in Notion/Cursor.
2. **Developer credentials** — API keys, SSH passphrases, DB passwords. Previously a slice of 1Password or manual Keychain Access.

These share a "stored text" shape but have **inverted security models**. Merging them would either lock snippets behind vault friction or strip secrets of locking.

## Decision

### Snippets (P0)

- Local-only plaintext JSON at `~/Library/Application Support/Luma/snippets.json`.
- Trigger: `s ` (also `snip `). Return copies; Tab pastes via Accessibility.
- Dashboard green card (column 1, row 1). Always available — no unlock.
- Detail view: Add / Edit / Delete / Duplicate in Route B same-panel container.
- Frecency ranking: fuzzy match + usage count + recency decay.

### Secrets (P0)

- Keychain-backed vault for developer credentials only. Labels/account in JSON metadata; values in Keychain service `app.luma.secrets`.
- Trigger: `secret ` / `secrets `. Locked by default.
- Dashboard gold card (column 2, row 1). Detail view CRUD when unlocked.
- Copy secret: auto-clear pasteboard after configurable delay (default 10 s) if unchanged.
- Re-lock after configurable idle timeout (default 5 min). Menu bar icon reflects lock state.
- Settings: auto-clear seconds, re-lock timeout, require-unlock-on-launch toggle.

### Explicit non-goals

- Merging Snippets and Secrets into one module.
- Browser website password autofill, TOTP, team sync, 1Password import.
- System-wide text expansion triggers.
- Cloud sync.

## Consequences

- Dashboard grows to 7 core cards (still ≤ 8 ceiling).
- `ActionKind.copyToPasteboardSecure` and `PasteboardClient.writeSecure` added for timed pasteboard clearing.
- `ConfigurationClient` extended with secrets settings keys.
