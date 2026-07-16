# ADR-0005: Records database and manually imported projects

- Status: Accepted
- Date: 2026-07-15

## Decision

Luma treats Records and manually imported Projects as local personal data features. They are
not public-product integrations and do not introduce an agent, watcher, or background service.

### Records

- `records.sqlite` under LumaNext is the long-term source of truth after import.
- `~/Documents/Notes/Records/*.md` (or the configured `records_root`) is an explicit, read-only
  import source. Luma never writes Markdown back.
- The MVP domain fields are category, name, rating (1–10 or empty), and note. `source_*` and
  timestamps are internal import metadata.
- `luma record import --root PATH` is dry-run; `--apply` snapshots the Records database, writes
  through the Records migration ledger, and leaves source files unchanged.
- Import is idempotent by canonical source identity plus normalized name. Existing database
  edits win when a source row changes. A user deletion is remembered as a tombstone so a later
  import does not silently resurrect it.
- TUI mutations use the Records application port. The module does not open or mutate
  `ConfigStore` directly.

### Projects

- Plain `proj` search contains only directories explicitly imported by the user.
- Import requires an existing canonical directory with no user-controlled symlink component and
  rejects duplicates. `imported_projects` has a serde default so old settings remain readable.
- Path validation, containment checks, and bounded directory enumeration are performed through
  the `ProjectWorkspacePort` macOS adapter; the Projects module does not perform filesystem I/O.
- `proj add/import`, `proj remove`, and `config set --import-project/--remove-project` mutate
  settings through the application settings CAS. Removing a project removes only its config
  entry; it never deletes the directory.

## Consequences

- Module-local `permission`, `unavailable`, and `not_configured` rows are the supported failure
  UX. There is no `doctor` command, overlay, diagnostics export, or probe-port subsystem.
- Tests must isolate LumaNext support/log paths and cover dry-run read-only behavior, CAS conflicts,
  path/symlink validation, import idempotency, parser edge cases, and rollback target isolation.
