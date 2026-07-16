# Codebase governance (personal)

Luma is a solo daily-driver. Governance here means **keeping the tree navigable and
honest**, not process theater. Prefer small friction fixes over roadmap completion.

Related: [MODULES.md](./MODULES.md), [ADR-0001](./adr/0001-rust-tui-product-shape.md),
[ADR-0002](./adr/0002-defer-luma-agent.md), `scripts/check_architecture.sh`.

## 1. What we govern vs what we ignore

| Govern | Do not govern |
| --- | --- |
| Product boundary (personal TUI workbench, no AI agent) | Public release, notarization, marketing |
| Crate layering + ports | Architecture purity refactors “for cleanliness” |
| Module inventory accuracy (`MODULES.md` ↔ `compose.rs`) | Stub-module growth (layouts / menu / browser tabs) |
| Honest failure outcomes | Centralized `doctor` / diagnostics export |
| Soft file-size / split triggers when touching hot paths | Long soak / PTY evidence packs / cargo-deny-as-policy |
| Doc drift that misleads daily use | Checklist completion for its own sake |

## 2. Standing rules

### Product & data

1. **Boundary** — CLI/TUI only; no Web/Tauri/Electron/agent daemon/LLM chat unless an ADR
   explicitly changes ADR-0001 / ADR-0002.
2. **Composition** — `bins/luma` (+ `compose.rs`) is the sole registration root for modules
   and adapters.
3. **Persistence** — live data under LumaNext only (`LUMA_NEXT_*` in tests). No silent writes
   outside that tree.
4. **Honesty** — permission / unavailable / not-configured / empty are distinct. Never fake
   success.

### Architecture

5. **Layering** — keep the allowlist green:
   ```bash
   cd rust && ./scripts/check_architecture.sh
   ```
   Modules do not call Mac APIs or open stores directly; they take ports/repos.
6. **TUI I/O** — `update` / `render` stay pure; Effects own I/O (ADR-0001).
7. **No doctor** — module-local status rows only.

### Docs that must stay true

8. When a module is added, removed, renamed, default-on/off changed, or triggers change,
   update in the **same change**:
   - `rust/docs/MODULES.md` (source of truth)
   - root `README.md` In/Out table (keep In list complete)
   - `AGENTS.md` / `.cursor/rules/personal-use.mdc` only if the product boundary moves
9. Deferred non-goals (Window layouts, Menu search, Browser tabs, signed-host Translate)
   stay **out of tree** — do not reintroduce as half-implemented stubs.

### File hygiene (soft limits)

These are **when-you-touch** rules, not a mandate to split everything now.

| Soft limit | Action when exceeded and you are already editing the area |
| --- | --- |
| Module production file ≳ 1200 lines | Prefer a subdirectory (`notes/`-style): domain helpers, actions, tests co-located or `*_tests` module |
| Platform/adapter file ≳ 1500 lines | Split parse/import vs store I/O (e.g. proxy profile YAML/URI vs persistence) |
| TUI `reducer.rs` / `view_model.rs` growth | Extract by message family or surface; keep tests next to the extracted unit |
| Single-file “directory” modules | Only keep `foo/mod.rs` alone if a second file is imminent; otherwise flat `foo.rs` is fine |

Do **not** open a PR whose only purpose is splitting a file you are not changing for a real fix.

### Naming (converge opportunistically)

10. Module constructors: prefer `with_deps(...)` when injecting ports/repos. Legacy
    `with_store` / `with_root` / `with_catalog` / `with_settings` may stay until that module
    is touched.
11. Platform adapters: prefer `Mac*` for macOS host I/O. Filesystem-only catalogs may keep
    descriptive names (`FilesystemAppsCatalog`) — document in the module’s port comment.

## 3. Change playbooks

### Add or materially change a module

1. Implement behind ports; register in `compose.rs`.
2. Update `MODULES.md` + root README In list.
3. Cover honesty paths (not_configured / unavailable / permission) and cancel where actions
   confirm.
4. Run the verify set below.

### Touch a mega-file

1. Ship the friction fix first.
2. If the file is over the soft limit, take **one** bounded extract in the same PR when it
   reduces the next edit’s cost; otherwise leave a one-line `// TODO(split): …` is **not**
   required — prefer doing nothing over drive-by TODOs.
3. Keep architecture allowlist green.

### Product-boundary change

1. New or amended ADR first (agent daemon, AI chat, stub revival, etc.).
2. Then code. Do not “quietly” grow out-of-scope surfaces.

## 4. Hygiene backlog (friction-ordered)

One-shot navigability cleanup (2026-07-16) completed the queued splits below. Ongoing rule:
prefer splits only when the area is already being changed, or when daily use is blocked by
navigability — do not open purity-only follow-ups.

| Priority | Item | Status |
| --- | --- | --- |
| P0 | Keep `MODULES.md` ↔ compose ↔ README In list in sync | Done (and standing rule §2) |
| P1 | Split `profile_store` into parse / fetch / clash / fs / store | Done → `profile_store/` |
| P1 | Carve wordbook review out of TUI `reducer` | Done → `reducer/wordbook.rs` |
| P2 | Subdirectory-split `proxy` / `ssh` / `projects` / `timers` | Done |
| P3 | `ProjectsModule::with_settings` → `with_deps` on touched module | Done (`with_roots` kept as thin helper) |
| — | Explicitly **not** queued | Stub revival, doctor, crate splits for purity, release gates |

## 5. Verify (enough for personal)

```bash
cd rust
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p luma --test cli_blackbox
./scripts/check_architecture.sh
```

No extra CI release gates. Architecture script failures are real; fix them in the same change.

## 6. Anti-patterns

- “Clean architecture” PRs with no personal-use benefit
- Reintroducing stubs to “complete the matrix”
- Central diagnostics / doctor overlays
- AI/agent/tool-loop product shape without an ADR
- Doc-only churn that does not correct a false claim
- Splitting files solely to hit a line-count aesthetic

## 7. Ownership

Solo maintainer. Agents and future-you follow this file + ADRs + `MODULES.md`. When this
doc disagrees with an Accepted ADR, the ADR wins until amended.
