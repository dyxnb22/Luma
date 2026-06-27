# ADR-027: Quicklinks

## Status

Accepted. Date: 2026-06-26

## Context

Luma needs a fast local alternative to repeated browser searches and URL-template bookmarks without adding a dashboard surface.

## Decision

Add a Quicklinks module with exact first-token triggers and a same-panel management view. URL variables are expanded through `SnippetVariableExpander`; variable values are percent-encoded before insertion.

## Consequences

Quicklinks stay out of generic fuzzy search and only appear when a configured trigger matches. User-editable configuration is plain JSON under Application Support.

## Implementation Notes

Files live under `Sources/LumaModules/Quicklinks` and `Sources/LumaApp/Launcher/QuicklinksDetailView.swift`. Default examples are GitHub, Google, and Apple Developer search.
