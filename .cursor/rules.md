# Cursor Rules for Luma

Precise implementation assistant. Scoped edits aligned with Route C.

## Read First

1. **`docs/ENGINEERING_PACKAGE.md`** — single development entry point (reading order, conflict priority, module/workbench rules).
2. **Frozen specs** — read before touching matching surfaces:
   - `docs/specs/LAUNCHER_HOME_CONSTRAINTS.md` (empty-query home)
   - `docs/specs/LAUNCHER_PANEL_CONSTRAINTS.md` (panel geometry, placement, in-panel layout)
   - `docs/specs/UX_BEHAVIOR_RULES.md` (navigation, keyboard, detail exit)
   - `docs/specs/NOTES_DETAIL_CONSTRAINTS.md` (Notes detail only)
3. **Open gaps:** `docs/qa/LAUNCHER_NAVIGATION_AUDIT.md`

Historical ADRs may describe dashboard cards or Route B home (superseded by ADR-023 / ADR-032). Do not implement those instructions unless the user explicitly asks for archaeology.

## Composer Rules

- Use normal mode only.
- Do not enable Fast mode.
- One scoped task at a time.
- No adjacent features unless explicitly requested.
- If a request conflicts with Route C or frozen specs, stop and ask.

## Verification

- `swift build`
- `swift test` for module, ranking, persistence, and action changes
- `./scripts/build_app.sh` for runtime or bundle changes
- `./scripts/qa/run_full_smoke.sh` for meaningful launcher UX changes

Do not claim completion without honest verification status.
