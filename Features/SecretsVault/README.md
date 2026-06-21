# Secrets Vault

## Goal

Store key information such as passwords, API keys, tokens, recovery codes, and server notes in a dedicated encrypted vault. This is separate from clipboard history.

## MVP Behavior

- Search only after explicit unlock.
- Result titles may show labels, never secret values.
- Actions:
  - Copy secret value.
  - Copy username/account.
  - Open edit sheet.
  - Rotate/mark stale later.

## Security Direction

- Current implementation includes a Keychain-backed store for secret values.
- Store metadata in SQLite later; secret values should stay in Keychain or encrypted payloads.
- Never include secret values in general ranking, logs, diagnostics, or previews.
- Auto-lock after inactivity.
- Pasteboard writes from this module should use short-lived clearing later.

## UI Card

- Locked state by default.
- Shows number of saved items and stale items.
- Edit button opens vault management after unlock.

## Implementation Entry

- Source module: `Sources/LumaModules/Secrets/SecretsModule.swift`
- Future service: `SecretsVaultService`
