//! Module-scoped list-focus key shortcuts consumed by `map_key`.
//!
//! Keep new bindings here (or on the module later) instead of growing special-case
//! arms in `app::map_key`.

/// `(module_id, &[(key, action_id)])` — Search route, non-prompt focus only.
pub const MODULE_LIST_SHORTCUTS: &[(&str, &[(char, &str)])] = &[(
    "luma.command_recipes",
    &[('r', "run"), ('c', "copy"), ('f', "favorite")],
)];

/// Resolve a list-focus shortcut for the selected module, if any.
pub fn list_shortcut_action(module_id: &str, key: char) -> Option<&'static str> {
    let key = key.to_ascii_lowercase();
    MODULE_LIST_SHORTCUTS
        .iter()
        .find(|(id, _)| *id == module_id)
        .and_then(|(_, bindings)| {
            bindings
                .iter()
                .find(|(c, _)| *c == key)
                .map(|(_, action)| *action)
        })
}
