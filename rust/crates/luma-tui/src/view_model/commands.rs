use super::{AppState, CommandPaletteEntry};

impl AppState {
    /// Slash-prefixed bare module trigger (`/clip`, not `clip` or `/clip `).
    /// Unprefixed input is always a global search under the strict command format.
    pub fn incomplete_slash_trigger(&self) -> Option<String> {
        let is_prefix = |token: &str| {
            matches!(token, "help" | "settings" | "commands" | "scroll" | "quit")
                || self
                    .module_catalog
                    .iter()
                    .any(|m| m.enabled && m.triggers.iter().any(|t| t.eq_ignore_ascii_case(token)))
        };
        let query =
            luma_domain::Query::parse_with_prefixes_strict(&self.search.prompt, 50, is_prefix);
        if !query.is_incomplete_trigger(is_prefix) {
            return None;
        }
        Some(
            luma_domain::strip_command_prefix(&self.search.prompt)
                .trim()
                .to_ascii_lowercase(),
        )
    }

    fn all_command_palette_rows(&self) -> Vec<CommandPaletteEntry> {
        let mut rows = vec![
            CommandPaletteEntry {
                id: "settings".into(),
                label: "/settings".into(),
                description: "Open workbench settings".into(),
                query: None,
                submit: true,
            },
            CommandPaletteEntry {
                id: "settings:projects-root".into(),
                label: "/settings projects-root <path>".into(),
                description: "Add a project browse root".into(),
                query: Some("/settings projects-root ".into()),
                submit: false,
            },
            CommandPaletteEntry {
                id: "settings:import-project".into(),
                label: "/settings import-project <path>".into(),
                description: "Import one existing project directory".into(),
                query: Some("/settings import-project ".into()),
                submit: false,
            },
            CommandPaletteEntry {
                id: "settings:records-root".into(),
                label: "/settings records-root <path|none>".into(),
                description: "Set or clear the Records Markdown source".into(),
                query: Some("/settings records-root ".into()),
                submit: false,
            },
            CommandPaletteEntry {
                id: "settings:clipboard-retention".into(),
                label: "/settings clipboard-retention-days <days>".into(),
                description: "Set clipboard history retention".into(),
                query: Some("/settings clipboard-retention-days ".into()),
                submit: false,
            },
            CommandPaletteEntry {
                id: "settings:secrets-lock".into(),
                label: "/settings secrets-idle-lock-secs <seconds>".into(),
                description: "Set the in-process Secrets idle lock".into(),
                query: Some("/settings secrets-idle-lock-secs ".into()),
                submit: false,
            },
            CommandPaletteEntry {
                id: "settings:hub-windows".into(),
                label: "/settings hub-windows-max <5-50>".into(),
                description: "Set the visible-window Hub cap".into(),
                query: Some("/settings hub-windows-max ".into()),
                submit: false,
            },
            CommandPaletteEntry {
                id: "scroll:up".into(),
                label: "/scroll up".into(),
                description: "Scroll the underlying focused surface up one page".into(),
                query: None,
                submit: true,
            },
            CommandPaletteEntry {
                id: "scroll:down".into(),
                label: "/scroll down".into(),
                description: "Scroll the underlying focused surface down one page".into(),
                query: None,
                submit: true,
            },
        ];
        let recent_modules = self
            .hub
            .continue_items
            .iter()
            .map(|item| item.module_id.as_str())
            .collect::<Vec<_>>();
        let mut modules = self
            .module_catalog
            .iter()
            .filter(|module| module.enabled)
            .collect::<Vec<_>>();
        modules.sort_by(|a, b| {
            let a_recent = recent_modules
                .iter()
                .position(|id| *id == a.id)
                .unwrap_or(usize::MAX);
            let b_recent = recent_modules
                .iter()
                .position(|id| *id == b.id)
                .unwrap_or(usize::MAX);
            a_recent.cmp(&b_recent).then_with(|| {
                a.display_name
                    .to_lowercase()
                    .cmp(&b.display_name.to_lowercase())
            })
        });
        for module in modules {
            if module.commands.is_empty() {
                let Some(query) = module.suggested_query.clone().or_else(|| {
                    module
                        .triggers
                        .first()
                        .map(|trigger| format!("/{trigger} "))
                }) else {
                    continue;
                };
                rows.push(CommandPaletteEntry {
                    id: format!("module:{}", module.id),
                    label: query.trim_end().to_string(),
                    description: format!("Open {}", module.display_name),
                    query: Some(query),
                    submit: true,
                });
                continue;
            }
            rows.extend(module.commands.iter().enumerate().map(|(index, command)| {
                let description = command
                    .example
                    .as_ref()
                    .map(|example| format!("{} · e.g. {example}", command.description))
                    .unwrap_or_else(|| command.description.clone());
                CommandPaletteEntry {
                    id: format!("module:{}:{index}", module.id),
                    label: command.syntax.clone(),
                    description,
                    query: Some(command.query.clone()),
                    submit: !command.syntax.contains('<'),
                }
            }));
        }
        rows.extend([
            CommandPaletteEntry {
                id: "help".into(),
                label: "/help".into(),
                description: "Keyboard and module help".into(),
                query: None,
                submit: true,
            },
            CommandPaletteEntry {
                id: "commands".into(),
                label: "/commands [filter]".into(),
                description: "Search this command palette".into(),
                query: None,
                submit: true,
            },
            CommandPaletteEntry {
                id: "quit".into(),
                label: "/quit".into(),
                description: "Stop the workbench session".into(),
                query: None,
                submit: true,
            },
        ]);

        rows
    }

    pub fn command_palette_rows(&self) -> Vec<CommandPaletteEntry> {
        let mut rows = self.all_command_palette_rows();
        let filter = self.overlay.commands_filter.trim().to_lowercase();
        if !filter.is_empty() {
            rows.retain(|row| {
                row.label.to_lowercase().contains(&filter)
                    || row.description.to_lowercase().contains(&filter)
            });
        }
        rows
    }

    /// Bounded structured completions for a partially typed slash command.
    pub fn command_completion_candidates(&self) -> Vec<String> {
        let input = self.search.prompt.trim().to_ascii_lowercase();
        if !input.starts_with('/') || !input.contains(char::is_whitespace) {
            return Vec::new();
        }
        let mut candidates = self
            .all_command_palette_rows()
            .into_iter()
            .filter(|entry| entry.label.starts_with('/'))
            .filter(|entry| {
                let syntax = entry.label.to_ascii_lowercase();
                let query = entry
                    .query
                    .as_deref()
                    .unwrap_or(entry.label.as_str())
                    .trim()
                    .to_ascii_lowercase();
                (syntax != input || entry.label.contains('<') || entry.label.contains('['))
                    && (syntax.starts_with(&input)
                        || (query.starts_with(&input) && query != input)
                        || (self
                            .search
                            .prompt
                            .chars()
                            .last()
                            .is_some_and(char::is_whitespace)
                            && syntax.starts_with(input.trim_end())))
            })
            .map(|entry| entry.label)
            .take(3)
            .collect::<Vec<_>>();
        candidates.dedup();
        candidates
    }

    pub fn help_lines(&self) -> Vec<String> {
        let mut lines = vec![
            "Enter opens a bare trigger (`/clip`) · unprefixed text is global search".into(),
            "Enter action · Ctrl-k actions · Ctrl-/ commands · Tab focus · S-Tab preview · ? help"
                .into(),
            format!(
                "{}{} move · PgUp/PgDn or /scroll up/down page · Ctrl-p/n history",
                self.symbols.up, self.symbols.down
            ),
            "Hub / win list: 1-9 focus visible window · Enter open".into(),
            String::new(),
            "Workbench commands:".into(),
        ];
        for command in self
            .all_command_palette_rows()
            .into_iter()
            .filter(|entry| !entry.id.starts_with("module:"))
        {
            lines.push(format!("  {} — {}", command.label, command.description));
        }
        lines.push(String::new());
        lines.push("Enabled module commands:".into());
        let mut modules = self
            .module_catalog
            .iter()
            .filter(|module| module.enabled)
            .collect::<Vec<_>>();
        modules.sort_by(|left, right| {
            left.display_name
                .to_ascii_lowercase()
                .cmp(&right.display_name.to_ascii_lowercase())
        });
        if modules.is_empty() {
            lines.push("  (waiting for session catalog)".into());
        } else {
            for module in modules {
                lines.push(format!("  {}:", module.display_name));
                if module.commands.is_empty() {
                    let query = module
                        .suggested_query
                        .as_deref()
                        .map(str::trim_end)
                        .unwrap_or("(no interactive command)");
                    lines.push(format!("    {query}"));
                    continue;
                }
                for command in &module.commands {
                    let example = command
                        .example
                        .as_ref()
                        .map(|example| format!(" · e.g. {example}"))
                        .unwrap_or_default();
                    lines.push(format!(
                        "    {} — {}{}",
                        command.syntax, command.description, example
                    ));
                }
            }
        }
        lines.push(String::new());
        lines.push("Confirm / Destructive actions always ask first.".into());
        lines
    }

    pub fn help_scroll_max(&self) -> usize {
        let preferred = self.terminal.height.saturating_sub(2).clamp(12, 22);
        let overlay_height = preferred.min(self.terminal.height.saturating_sub(4)).max(5);
        let visible = overlay_height.saturating_sub(2) as usize;
        self.help_lines().len().saturating_sub(visible.max(1))
    }
}
