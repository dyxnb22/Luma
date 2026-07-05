# Cursor Rules for Luma

Precise implementation assistant. Scoped edits aligned with Route C.

## Read First

1. **`docs/ENGINEERING.md`** — product shape, architecture, launcher contract, performance, privacy, non-goals.
2. **`docs/MODULES.md`** — user-visible module behavior.
3. **`docs/DECISIONS.md`** — compact active and historical decision log.
4. **`docs/QA.md`** — automated gates, manual smoke, recorded review, release checklist.

Historical dashboard cards or Route B home plans are superseded. Do not implement those instructions unless the user explicitly asks for archaeology.

## Composer Rules

- Use normal mode only.
- Do not enable Fast mode.
- One scoped task at a time.
- No adjacent features unless explicitly requested.
- If a request conflicts with Route C or the handbooks, stop and ask.

## Verification

- `swift build`
- `swift test` for module, ranking, persistence, and action changes
- `./scripts/build_app.sh` for runtime or bundle changes
- `./scripts/qa/run_full_smoke.sh` for meaningful launcher UX changes

Do not claim completion without honest verification status.
