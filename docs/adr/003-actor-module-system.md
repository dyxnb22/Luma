# ADR-003: Actor-Based Module System

Status: Accepted

## Context

Each module owns mutable indexes and must not block other modules or UI.

## Decision

Concrete modules are actors behind `LumaModule`. `QueryDispatcher` fans out with task groups and timeouts.

## Consequences

State ownership is clear. Sendable boundaries must be respected from day one.
