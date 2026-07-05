# v1 Release Readiness QA — 2026-07-05

Automated verification from the v1 release readiness implementation pass.

## Automated

| Check | Result |
| --- | --- |
| `swift test` | Pass (596 tests) |
| `swift test --filter BrowserTabs` | Pass |
| `swift test --filter ActionExecutor` | Pass |
| `./scripts/build_app.sh --no-restart` | Pass |
| `LUMA_SKIP_NOTARIZATION=1 ./scripts/release/build_dmg.sh` | Run before tag (requires signing identity) |

## Manual (required before v1 tag)

- [ ] `./scripts/qa/run_full_smoke.sh` on a restored QA environment
- [ ] VoiceOver walkthrough (search, list rows, detail Esc stack, Settings)
- [ ] Clean-user Gatekeeper install from notarized DMG (`spctl -a -vv`)

## Notes

- Browser Tabs ADR-030 amended to stale-while-revalidate; first cold query may return zero tab rows until background refresh completes.
- Commands module remains default-off; Settings copy now mentions `commands.json` local scripts.
- i18n wave 1 covers Clipboard, Notes detail chips/panels, Browser Tabs diagnostics/actions, Secrets detail.
