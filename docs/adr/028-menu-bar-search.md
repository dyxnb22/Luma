# ADR-028: Menu Bar Search

## Status

Accepted. Date: 2026-06-26

## Context

Many macOS commands are only discoverable through the active app menu. Luma can make them keyboard-searchable, but AX traversal is too slow for the keystroke path.

## Decision

Add a service-backed Menu Items module. `MenuBarTreeService` refreshes a bounded AX cache on app activation; `MenuItemsModule.handle()` only reads cached records and fuzzy-scores title paths.

## Consequences

Accessibility permission is required. Cache freshness is app-scoped with a short TTL, and disabled bundle IDs are configurable.

## Implementation Notes

The module uses `mb` / `menu` triggers and stores optional config at `~/Library/Application Support/Luma/menu-items.json`. `MenuItemPresser` re-resolves the AX path during `perform()`.
