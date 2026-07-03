# Module Maturity Checklist

Use this when adding or shipping changes to a built-in module. Goal: consistent command-first behavior without expanding launcher hot-path scope.

## Registration

- [ ] `*ModuleBundle.swift` registered in `ModuleRegistry.allBundles`
- [ ] `warmupTier` matches behavior (`hotPath` only if `handle` is memory-only and fast)
- [ ] `ModuleDetailRegistry` factory when the module has in-panel detail
- [ ] `Features/<Module>/README.md` documents triggers, detail entry, and permission needs

## Query path

- [ ] Stable `ResultID` keys for all rows
- [ ] `handle` does not read disk or network (indexes loaded in `warmup` / FSEvents)
- [ ] Targeted prefix queries work (`<trigger> `) with help row (`<trigger> ?` / `help <trigger>`)
- [ ] Bare/open-detail row uses `rowKind: .actionable` and opens detail on Return
- [ ] Disabled module surfaces `ModuleDiagnosticResults` row, not silent empty

## Actions & detail

- [ ] Custom actions use `ModuleActionCoding` payloads; `perform` respects 2s soft budget
- [ ] Detail shortcuts work when subviews hold focus (see `LAUNCHER_NAVIGATION_AUDIT.md` MOD-KB)
- [ ] Detail layout follows `LAUNCHER_PANEL_CONSTRAINTS.md` (no full-width `wantsLayer`; scroll inside right column when opened from empty home — ADR-032)
- [ ] Notes detail changes follow `NOTES_DETAIL_CONSTRAINTS.md` when applicable
- [ ] User-facing strings use `L10n.tr` with entries in `L10nStrings.json`

## Workbench (if applicable)

- [ ] Capture targets exposed via narrow draft builders, not App-layer model construction
- [ ] Command preview rows are side-effect free; execution only on Return
- [ ] Disabled modules omitted from workbench command/detail surfaces

## Tests

- [ ] `Tests/LumaModulesTests/<Module>*Tests.swift` covers parse, store, and primary actions
- [ ] Permission-required paths return diagnostic rows or clear status copy
- [ ] `swift test` green before merge

## Default-off modules

Commands, Records (`luma.media`), Browser Tabs, and Auto Workflow ship **disabled**. Each must include `defaultOffNote` on the bundle and Settings copy explaining why.
