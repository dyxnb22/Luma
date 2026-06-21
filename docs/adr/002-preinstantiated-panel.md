# ADR-002: Pre-Instantiated Launcher Panel

Status: Accepted

## Context

Creating a panel during hotkey handling risks visible latency.

## Decision

Create `LauncherPanel` at app launch, lay it out once, keep it ordered out until needed.

## Consequences

Slight idle memory cost in exchange for fast reveal.
