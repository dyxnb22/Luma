# ADR-004: In-Process Modules for v1

Status: Accepted

## Context

External plugin APIs create ABI, signing, lifecycle, and security costs before the module shape is proven.

## Decision

Compile v1 modules into the app. Add scripted commands before considering XPC plugins.

## Consequences

Simpler development and debugging. Crash isolation waits until a real external plugin need exists.
