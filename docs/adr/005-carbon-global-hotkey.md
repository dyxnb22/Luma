# ADR-005: Carbon Global Hotkey

Status: Accepted

## Context

`NSEvent.addGlobalMonitorForEvents` observes keys but does not consume a launcher chord.

## Decision

Use a thin wrapper around `RegisterEventHotKey`.

## Consequences

Some old Carbon API surface remains, but behavior matches launcher expectations.
