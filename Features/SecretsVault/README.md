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
- Metadata is persisted as JSON at `~/Library/Application Support/Luma/secrets-metadata.json`.
- Store metadata in SQLite later only if JSON becomes too limited; secret values should stay in Keychain or encrypted payloads.
- Never include secret values in general ranking, logs, diagnostics, or previews.
- Auto-lock after inactivity.
- Pasteboard writes from this module should use short-lived clearing later.

## Current UX

- `secret` / `sec` opens the same-panel Secrets detail view.
- Locked state is the default; unlocked state exposes search plus CRUD actions.
- Search results never reveal secret values in row titles or subtitles.
- There is no dashboard card in Route C.

## Implementation Entry

- Source module: `Sources/LumaModules/Secrets/SecretsModule.swift`
- Vault implementation: `Sources/LumaModules/Secrets/SecretsVault.swift`
- Keychain value store: `Sources/LumaModules/Secrets/KeychainSecretsStore.swift`
