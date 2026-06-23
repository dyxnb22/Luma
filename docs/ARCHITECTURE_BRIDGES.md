# LauncherBridge vs ModuleDetailRegistry

## ModuleDetailRegistry (`LumaApp` only)

**Purpose:** Detail views read shared module instances and build `ModuleDetailView` implementations.

- Module references: `clipboardModule`, `notesModule`, `snippetsModule`, `wordbookStore`, etc.
- `config`, `translation` for detail view construction
- `make(for: ModuleIdentifier)` factory
- `isLauncherQueryEmpty` launcher ↔ detail coordination flag

Detail views and `LauncherRootView` use this registry. **Modules must not import or call it.**

Prefer populating registry fields via `LauncherEnvironment.applyToModuleDetailRegistry()` at startup.

## LauncherBridge (`LumaModules`, injected by `LumaApp`)

**Status:** `@available(*, deprecated)` — use `LauncherEnvironment` for new code.

**Purpose:** Actor-isolated modules invoke main-actor UI callbacks without importing AppKit.

- `openModuleDetail(ModuleIdentifier)` — opens detail in-panel (Media, Wordbook, …)
- `reloadMediaDetail`, `reloadSecretsDetail`, `reloadSnippetsDetail`
- `onSecretsLockStateChanged`
- `onBackFromDetail`, `onOpenSettings`, `onTranslateContentChanged` (detail navigation)

`LauncherEnvironment.wireLegacyBridge()` mirrors env closures into `LauncherBridge` during the migration period.

`AppCoordinator.start()` constructs one `LauncherEnvironment` and wires callbacks once. New modules: extend `LauncherEnvironment`, not ad-hoc statics.

## Removed

- `openWordbookReview` — deleted (ADR-013). Wordbook uses `openModuleDetail(.wordbook)`.
- `openMediaDetail` — replaced by `openModuleDetail(.media)`.
