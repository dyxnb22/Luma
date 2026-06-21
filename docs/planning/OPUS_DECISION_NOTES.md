# Opus Decision Notes

## Strong Decisions to Preserve

- Use Swift + AppKit for the launcher.
- Keep the launcher panel pre-instantiated.
- Use actor-based in-process modules for v1.
- Protect every module query with a timeout.
- Measure latency from Phase 0.
- Defer public plugin APIs.
- Defer custom file indexing forever; use Spotlight when file search arrives.

## Resolved Decision

The original brief listed TODO and Translation as MVP features. Opus resolved the conflict: Calculator is the v1 MVP module; TODO and Translation move to v1.1.

Current skeleton includes extension points:

- `CalculatorModule` as a v1 module.
- `TodoModule` as a disabled v1.1 stub.
- `TranslationService` as a future service boundary.

## Local Skeleton Choice

The current default-enabled module set should be:

1. Apps
2. Windows
3. Clipboard
4. Commands
5. Calculator

`TodoModule` remains registered as a disabled stub. Translation is represented as a service boundary, not a module yet.
