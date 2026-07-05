# ADR-030: Browser Tabs Search

## Status

Amended (2026-07-05). Supersedes the empty/stale await-refresh clause in the original decision (2026-06-26).

## Context

Open browser tabs are high-value navigation targets, but AppleScript calls are slow and trigger Automation permission prompts.

## Decision

Add Browser Tabs as a default-off module. Browser adapters fetch tab records through AppleScript with an 800 ms timeout and a five-second cache.

`handle()` and warmup use the **stale-while-revalidate** pattern documented in [PERFORMANCE.md](../specs/PERFORMANCE.md): return cached tab records immediately and schedule background refresh when the cache is empty or past TTL. The query path must not await AppleScript on the keystroke hot path.

Module diagnostics (automation denied, timeout, degraded) surface as informational launcher rows via `QueryDispatcher`. When the cache is empty on first query, users may see no tab rows until background refresh completes; diagnostics rows carry permission/degraded state instead of blocking the query.

The 900 ms `queryTimeout` in the module manifest applies to adapter/refresh work, not to blocking `handle()`.

## Consequences

Users must enable the module and approve Automation per browser. Stale data is possible between refreshes, but this protects the 30 ms keystroke budget. First query after cold start may return zero tab rows while refresh runs in the background.

## Implementation Notes

Adapters cover Safari plus Chromium-family browsers: Chrome, Brave, Edge, and Arc. Parser logic is TSV-based and unit-tested independently from AppleScript.

`BrowserTabsService.searchableTabs()` returns `cached` immediately and calls `scheduleBackgroundRefreshIfNeeded()` when empty or stale.
