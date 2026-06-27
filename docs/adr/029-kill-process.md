# ADR-029: Kill Process

## Status

Accepted. Date: 2026-06-26

## Context

The existing app memory surface can quit apps, but users also need a direct command for quitting or relaunching visible GUI apps.

## Decision

Add `KillProcessModule` backed by `RunningProcessService`. It lists `NSWorkspace` regular applications, filters Luma, and exposes quit, force kill, and relaunch actions.

## Consequences

The module intentionally excludes daemons and raw signal handling in v1. Force kill requires the second modifier, and sensitive system bundle IDs require Return confirmation.

## Implementation Notes

Triggers are `kill`, `quit`, and `k`. The hot path searches cached process records and formats resident memory captured during warmup or after actions.
