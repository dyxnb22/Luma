# ADR-030: Browser Tabs Search

## Status

Accepted. Date: 2026-06-26

## Context

Open browser tabs are high-value navigation targets, but AppleScript calls are slow and trigger Automation permission prompts.

## Decision

Add Browser Tabs as a default-off module. Browser adapters fetch tab records through AppleScript with an 800 ms timeout and a five-second cache. `handle()` searches cached records and awaits refresh when the cache is empty or stale (see F-F-04).

Module diagnostics (automation denied, timeout, degraded) surface as informational launcher rows via `QueryDispatcher`.

## Consequences

Users must enable the module and approve Automation per browser. Stale data is possible between refreshes, but this protects the 30 ms keystroke budget.

## Implementation Notes

Adapters cover Safari plus Chromium-family browsers: Chrome, Brave, Edge, and Arc. Parser logic is TSV-based and unit-tested independently from AppleScript.
