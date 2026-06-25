# Performance Requirements

## Latency Targets

| Metric | p50 | p95 | Hard ceiling |
| --- | ---: | ---: | ---: |
| Hotkey press -> interactive panel | 25 ms | 50 ms | 80 ms |
| Hotkey press -> home list rendered (Route C) | 30 ms | 50 ms | 80 ms |
| Keystroke -> first ranked snapshot painted | 12 ms | 30 ms | 60 ms |
| Module `handle` call | 5-30 ms | <= timeout | 80 ms |
| `NotesModule.handle` | ≤ 10 ms | ≤ 30 ms | 40 ms (manifest queryTimeout) |
| `SnippetsModule.handle` | ≤ 10 ms | ≤ 20 ms | 30 ms (manifest queryTimeout) |
| `SecretsModule.handle` | ≤ 10 ms | ≤ 30 ms | 40 ms (manifest queryTimeout) |
| `MediaModule.handle` | ≤ 10 ms | ≤ 20 ms | 30 ms (manifest queryTimeout) |
| Media JSON warmup load | — | ≤ 80 ms | — (≤ 5000 items) |
| Panel hide after action | 10 ms | 20 ms | 40 ms |
| `WordbookDetailView.activate` | ≤ 50 ms | ≤ 100 ms warm / ≤ 200 ms cold | — |
| `AppIndex.search` (warm, ~1k apps) | — | ≤ 5 ms | — |
| `WordbookSessionPlanner.nextCard` (warm) | — | ≤ 30 ms | — |

Launcher convergence strategy adds a stricter working rule: warm keystroke p95 above 30 ms is a regression and should block the change.

## Hot Path Rules

- Panel is instantiated at app launch.
- Result rows use stable dimensions.
- No disk or network I/O in per-keystroke module queries.
- No animated table diffs.
- App icons are cached at display size.
- Query tasks are cancelled on every new keystroke.
- Panel hides before action completion.
- Notes module reads happen only in `warmup` and on FSEvents callbacks. `handle` never touches disk.
- `n doctor` is the on-demand health surface for frontmatter, duplicate names, and broken wiki links; it may read note bodies but is not on the keystroke hot path.
- Notes warmup duration is reported in `n doctor` stats and tracked in `NotesModule.lastWarmupMilliseconds()`.

## Measurement

Instrument these intervals first:

- `panel.show`.
- `home.render` (Route C empty-query first paint; tracked by `HomeLatencyTracker`).
- `query.keystroke_to_dispatch`.
- `query.dispatch`.
- `module.<id>.handle`.
- `ranker.score`.
- `result.render`.
- `action.execute`.

Phase 0 includes a debug latency HUD or a minimal log surface for p50/p95 while dogfooding.
