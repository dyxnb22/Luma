# ADR-025: Scripted Commands

Status: accepted  
Date: 2026-06-26

## Context

`CommandsModule` only exposed built-in settings/reload/quit actions. Users need local shell workflows without a plugin runtime.

## Decision

Load `commands.json` from Application Support in `CommandsModule.warmup`. Execute via `ScriptRunnerService` in `perform()` with timeout, background execution, and `UNUserNotification` on completion.

Extend existing `CommandsModule` rather than adding a separate module.

## Consequences

- No stdout streaming UI in v1.
- No remote script fetch or AppleScript runtime.
- Template variables share the snippet expander subset.
