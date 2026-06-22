# Performance Requirements

## Latency Targets

| Metric | p50 | p95 | Hard ceiling |
| --- | ---: | ---: | ---: |
| Hotkey press -> interactive panel | 25 ms | 50 ms | 80 ms |
| Keystroke -> first ranked snapshot painted | 12 ms | 30 ms | 60 ms |
| Module `handle` call | 5-30 ms | <= timeout | 80 ms |
| Panel hide after action | 10 ms | 20 ms | 40 ms |

Launcher convergence strategy adds a stricter working rule: warm keystroke p95 above 30 ms is a regression and should block the change.

## Hot Path Rules

- Panel is instantiated at app launch.
- Result rows use stable dimensions.
- No disk or network I/O in per-keystroke module queries.
- No animated table diffs.
- App icons are cached at display size.
- Query tasks are cancelled on every new keystroke.
- Panel hides before action completion.

## Measurement

Instrument these intervals first:

- `panel.show`.
- `query.keystroke_to_dispatch`.
- `query.dispatch`.
- `module.<id>.handle`.
- `ranker.score`.
- `result.render`.
- `action.execute`.

Phase 0 includes a debug latency HUD or a minimal log surface for p50/p95 while dogfooding.
