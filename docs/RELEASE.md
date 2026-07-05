# Luma Release Guide

This document describes how to produce a Developer ID signed, notarized DMG for v1 distribution.

## Prerequisites

- macOS 14+ build host with Xcode command-line tools (`xcode-select --install`)
- Apple Developer Program membership
- **Developer ID Application** certificate installed in the login keychain
- App-specific password or notary keychain profile for `notarytool`

## Environment variables

| Variable | Required | Purpose |
| --- | --- | --- |
| `LUMA_CODESIGN_IDENTITY` | Recommended | Explicit signing identity (defaults to Developer ID, then Apple Development, then local dev cert) |
| `NOTARY_PROFILE` | Notary (option A) | Keychain profile from `xcrun notarytool store-credentials` |
| `APPLE_ID` | Notary (option B) | Apple ID email |
| `APPLE_TEAM_ID` | Notary (option B) | Team ID |
| `APPLE_APP_SPECIFIC_PASSWORD` | Notary (option B) | App-specific password |
| `LUMA_SKIP_NOTARIZATION` | Optional | Set to `1` to build a signed DMG without notarization (local testing only) |
| `LUMA_WARNINGS_AS_ERRORS` | Optional | Set to `1` to fail release builds on Swift warnings |

## Tracked release metadata

- [Resources/Info.plist](../Resources/Info.plist) — bundle version, usage descriptions, `LSUIElement`
- [Resources/Luma.entitlements](../Resources/Luma.entitlements) — hardened runtime entitlements (Apple Events for Browser Tabs / AppleScript)

Update `CFBundleShortVersionString` and `CFBundleVersion` before each release tag.

## Local development build

```bash
./scripts/build_app.sh --no-restart
```

Produces `build/Luma.app` signed with the best available identity (Developer ID, Apple Development, or local dev cert).

## Release DMG

```bash
# One-time: store notary credentials
xcrun notarytool store-credentials "luma-notary" \
  --apple-id "you@example.com" \
  --team-id "TEAMID" \
  --password "app-specific-password"

export NOTARY_PROFILE=luma-notary
export LUMA_CODESIGN_IDENTITY="Developer ID Application: Your Name (TEAMID)"

./scripts/release/build_dmg.sh
```

Output: `build/release/Luma.dmg`

### Signed-only DMG (no notarization)

```bash
LUMA_SKIP_NOTARIZATION=1 ./scripts/release/build_dmg.sh
```

## Verification

```bash
spctl -a -vv build/Luma.app
codesign --verify --verbose=2 build/Luma.app
```

Notarized builds should report `source=Notarized Developer ID`.

## Release checklist

### Build machine

- [ ] Clean working tree at release tag
- [ ] `swift test` passes (596+ tests)
- [ ] `./scripts/release/build_dmg.sh` succeeds
- [ ] `spctl -a -vv build/Luma.app` accepted

### Clean machine (Gatekeeper)

- [ ] Mount DMG, drag Luma to Applications
- [ ] First launch passes Gatekeeper without quarantine override
- [ ] Global hotkey opens launcher
- [ ] Reminders permission prompt (Todo module)
- [ ] Accessibility banner appears only when AX modules enabled

### Accessibility (manual)

- [ ] VoiceOver: search field, list rows, detail enter/exit, Settings navigation
- [ ] Keyboard-only: Esc stack, Tab blocked in detail, module shortcuts in detail subviews

### Archive

- [ ] Record smoke run under `qa/<date>/` via `./scripts/qa/run_full_smoke.sh`
- [ ] Attach DMG checksum and test notes to release notes

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| `notarytool` authentication failed | Regenerate app-specific password; verify team ID |
| Gatekeeper rejects app | Ensure staple succeeded; re-submit if signing changed after notarization |
| Accessibility lost after rebuild | Use stable signing identity; run `./scripts/repair_accessibility_permission.sh` |
| Swift warnings in release build | Fix warnings or omit `LUMA_WARNINGS_AS_ERRORS=1` until clean |
