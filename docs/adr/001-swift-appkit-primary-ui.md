# ADR-001: Swift + AppKit for Primary UI

Status: Accepted

## Context

The launcher needs precise focus handling, low-latency keyboard response, stable panel behavior, and predictable rendering.

## Decision

Use Swift 6 and AppKit for the launcher panel and result list. SwiftUI is allowed for Settings/About only.

## Consequences

The hot path has more boilerplate, but avoids SwiftUI panel/focus surprises.
