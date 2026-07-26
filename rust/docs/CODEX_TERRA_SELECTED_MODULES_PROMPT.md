# Archived prompt for Codex GPT-5.6 Terra

Status: archived after all eight modules were implemented and verified. This file preserves the
original handoff for provenance; do not run it as a new implementation request. Current behavior
and constraints live in `MODULES.md` and `SELECTED_MODULES_PLAN.md`.

## Historical handoff

```text
You are implementing eight approved modules in the existing Luma repository. Work directly in the
current workspace and continue autonomously until the requested scope is complete or a concrete
technical blocker prevents safe progress.

First read these files completely before editing:

- AGENTS.md
- rust/docs/GOVERNANCE.md
- rust/docs/MODULES.md
- rust/docs/SELECTED_MODULES_PLAN.md
- the accepted ADRs linked by GOVERNANCE.md that affect product shape, module boundaries, or the
  native workbench host

Treat rust/docs/SELECTED_MODULES_PLAN.md as the feature specification and coding guide. Implement
the modules in its prescribed order:

1. Calculator
2. Downloads Inbox
3. Packages
4. Apple Shortcuts Bridge
5. Shell Recall
6. Renewals
7. Database Portals
8. Screen OCR

Important execution rules:

- Finish one vertical slice before starting the next. Do not create eight empty/scaffold modules.
- Preserve the existing dirty worktree and do not overwrite or revert unrelated user changes.
- Keep all module commands slash-prefixed; bare text remains global search.
- Keep bins/luma/src/compose.rs as the sole composition root.
- Put platform/process/filesystem/native work behind application ports and persistence behind
  repositories/storage adapters. Modules must not call macOS APIs or open SQLite directly.
- Do not add module-specific protocol Command/Event variants or central Engine dispatch arms.
- Do not add AI/LLM chat, agents, task loops, background daemons, a Doctor surface, release
  packaging, or deferred Window layouts/Menu/Browser tabs.
- Do not move OCR or any module UI/data into the Swift native workbench host.
- Use explicit program + args; never interpolate commands through a shell.
- Make cancellation real and require confirmation plus live identity revalidation for unsafe
  actions.
- Tests must use fakes/temp paths and must not open Finder, steal focus, mutate the real
  pasteboard, run Homebrew mutations, capture the real screen, connect to a real database, or
  modify real shell history.
- Do not weaken an acceptance rule just to get a test green.

For each module:

1. Inspect the closest existing module, port, adapter, repository, composition, CLI, and blackbox
   patterns before writing code. Reuse conventions; do not perform architecture cleanup for its
   own sake.
2. Implement the application port/repository, fake, platform/storage adapter, module, exports,
   composition, and any strictly necessary CLI provisioning.
3. Cover happy, empty/not-configured, unavailable/permission, failure, stale identity,
   confirmation, privacy, capacity, and cancellation paths applicable to that module.
4. Run focused tests, then the complete verification set below.
5. Only after the module is real and green, update rust/docs/MODULES.md and the root README In list
   in the same change. Never mark later unimplemented modules Available.
6. Review git diff for accidental scope growth before continuing.

Full verification:

cd rust
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p luma --test cli_blackbox
./scripts/check_architecture.sh

Special stop conditions:

- Database Portals MVP must not store or inject PostgreSQL passwords. Use existing libpq auth or an
  interactive psql prompt exactly as specified. Do not invent secret transport through argv,
  action payload, Recall, metadata SQLite, logs, or previews.
- Before registering Screen OCR, prove an isolated macOS adapter can safely use system region
  capture plus Apple Vision, clean temporary images on every path, and keep the Swift host thin.
  If that cannot be done safely, leave the prior seven modules green, document the exact blocker,
  and stop rather than substituting cloud OCR or moving logic into Swift.
- If a requirement conflicts with an accepted ADR or current architecture, report the exact
  conflict with file/line evidence and choose the smallest compliant implementation. Do not
  silently change product boundaries.

At the end, provide:

- modules completed and any intentionally deferred subfeatures;
- files changed, organized by module;
- focused and full verification results;
- manual macOS checks still required;
- any blocker with exact evidence.
```

## Archive note

For later maintenance, begin from the current diff and the implemented-module contract. Do not use
this historical prompt to scaffold or reimplement modules that already exist.
