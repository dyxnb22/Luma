//! Command shelf for SSH Workspace (keyboard-driven Copy/Insert).

use luma_domain::{
    render_remote_command, shell_quote, Recipe, RecipeMetadata, RecipeParameter,
    RecipeParameterKind, RecipeScope, RecipeTarget, SshRecipeContext,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShelfItemKind {
    SshNative {
        id: &'static str,
    },
    RemoteCommand {
        recipe_id: Option<String>,
        program: String,
        args: Vec<String>,
        parameters: Vec<RecipeParameter>,
        risk: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShelfItem {
    pub group: String,
    pub title: String,
    pub description: String,
    pub favorite: bool,
    pub use_count: u64,
    pub kind: ShelfItemKind,
}

#[derive(Clone, Debug, Default)]
pub struct ShelfState {
    pub items: Vec<ShelfItem>,
    pub filtered: Vec<usize>,
    pub selected: usize,
    pub filter: String,
    pub preview: Option<String>,
    pub param_drafts: BTreeMap<String, String>,
    pub param_order: Vec<String>,
    pub param_index: usize,
    pub filling_params: bool,
    /// When true, `/` filter mode receives typed characters.
    pub filter_editing: bool,
}

impl ShelfState {
    pub fn from_recipes(recipes: &[Recipe], include_static: bool) -> Self {
        Self::from_recipes_with_meta(recipes, &BTreeMap::new(), include_static)
    }

    pub fn from_recipes_with_meta(
        recipes: &[Recipe],
        meta: &BTreeMap<String, RecipeMetadata>,
        include_static: bool,
    ) -> Self {
        let mut items = Vec::new();
        if include_static {
            items.extend(static_ssh_ops());
        }
        for recipe in recipes.iter().filter(|r| {
            r.enabled && r.scope == RecipeScope::SshSession && r.target == RecipeTarget::RemoteShell
        }) {
            let Some(variant) = recipe.variants.first() else {
                continue;
            };
            let Some(step) = variant.steps.first() else {
                continue;
            };
            let recipe_meta = meta.get(&recipe.id).cloned().unwrap_or_default();
            items.push(ShelfItem {
                group: if recipe.group.is_empty() {
                    "Remote".into()
                } else {
                    recipe.group.clone()
                },
                title: recipe.title.clone(),
                description: recipe.description.clone(),
                favorite: recipe_meta.favorite,
                use_count: recipe_meta.use_count,
                kind: ShelfItemKind::RemoteCommand {
                    recipe_id: Some(recipe.id.clone()),
                    program: step.program.clone(),
                    args: step.args.clone(),
                    parameters: recipe.parameters.clone(),
                    risk: recipe.risk.as_str().into(),
                },
            });
        }
        if items
            .iter()
            .all(|i| matches!(i.kind, ShelfItemKind::SshNative { .. }))
            && recipes.is_empty()
        {
            items.extend(static_fallback_commands());
        }
        // Favorites first, then higher use_count, then title.
        let static_end = items
            .iter()
            .position(|i| matches!(i.kind, ShelfItemKind::RemoteCommand { .. }))
            .unwrap_or(items.len());
        let (ops, remotes) = items.split_at_mut(static_end);
        remotes.sort_by(|a, b| {
            b.favorite
                .cmp(&a.favorite)
                .then(b.use_count.cmp(&a.use_count))
                .then(a.title.cmp(&b.title))
        });
        let _ = ops;
        let mut shelf = Self {
            filtered: (0..items.len()).collect(),
            items,
            ..Self::default()
        };
        shelf.refilter();
        shelf
    }

    pub fn refilter(&mut self) {
        let needle = self.filter.to_ascii_lowercase();
        self.filtered = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                if needle.is_empty() {
                    return true;
                }
                item.title.to_ascii_lowercase().contains(&needle)
                    || item.group.to_ascii_lowercase().contains(&needle)
                    || item.description.to_ascii_lowercase().contains(&needle)
            })
            .map(|(idx, _)| idx)
            .collect();
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
        self.clear_form();
    }

    fn clear_form(&mut self) {
        self.preview = None;
        self.filling_params = false;
        self.param_drafts.clear();
        self.param_order.clear();
        self.param_index = 0;
    }

    pub fn selected_item(&self) -> Option<&ShelfItem> {
        self.filtered
            .get(self.selected)
            .and_then(|idx| self.items.get(*idx))
    }

    pub fn selected_item_mut(&mut self) -> Option<&mut ShelfItem> {
        let idx = *self.filtered.get(self.selected)?;
        self.items.get_mut(idx)
    }

    pub fn select_next(&mut self) {
        if self.filtered.is_empty() || self.filling_params {
            return;
        }
        self.selected = (self.selected + 1).min(self.filtered.len() - 1);
        self.clear_form();
    }

    pub fn select_prev(&mut self) {
        if self.filling_params {
            return;
        }
        self.selected = self.selected.saturating_sub(1);
        self.clear_form();
    }

    pub fn toggle_favorite_selected(&mut self) -> Option<(String, bool)> {
        let item = self.selected_item_mut()?;
        let ShelfItemKind::RemoteCommand {
            recipe_id: Some(id),
            ..
        } = &item.kind
        else {
            return None;
        };
        let id = id.clone();
        item.favorite = !item.favorite;
        let favorite = item.favorite;
        Some((id, favorite))
    }

    pub fn bump_use_count_selected(&mut self) -> Option<String> {
        let item = self.selected_item_mut()?;
        let ShelfItemKind::RemoteCommand {
            recipe_id: Some(id),
            ..
        } = &item.kind
        else {
            return None;
        };
        let id = id.clone();
        item.use_count = item.use_count.saturating_add(1);
        Some(id)
    }

    pub fn begin_preview_or_params(&mut self, ssh: &SshRecipeContext) -> Option<String> {
        let item = self.selected_item()?.clone();
        match item.kind {
            ShelfItemKind::SshNative { id } => {
                let text = native_text(id, ssh);
                self.preview = Some(text.clone());
                Some(text)
            }
            ShelfItemKind::RemoteCommand {
                program,
                args,
                parameters,
                ..
            } => {
                if !parameters.is_empty() && !self.params_ready(&parameters) {
                    if !self.filling_params {
                        self.start_param_form(&parameters, &item.title);
                    } else {
                        self.refresh_param_preview();
                    }
                    return self.preview.clone();
                }
                match render_remote_command(&program, &args, &parameters, &self.param_drafts, ssh) {
                    Ok(cmd) => {
                        self.filling_params = false;
                        self.preview = Some(cmd.clone());
                        Some(cmd)
                    }
                    Err(err) => {
                        self.preview = Some(err.to_string());
                        None
                    }
                }
            }
        }
    }

    fn params_ready(&self, parameters: &[RecipeParameter]) -> bool {
        if parameters.is_empty() {
            return true;
        }
        // Form must have been started (drafts populated) and required fields filled.
        if self.param_drafts.is_empty() && self.param_order.is_empty() {
            return false;
        }
        parameters.iter().all(|p| {
            let value = self
                .param_drafts
                .get(&p.id)
                .cloned()
                .or_else(|| p.default.clone())
                .unwrap_or_default();
            !p.required || !value.is_empty()
        })
    }

    fn start_param_form(&mut self, parameters: &[RecipeParameter], title: &str) {
        self.filling_params = true;
        self.filter_editing = false;
        self.param_order = parameters.iter().map(|p| p.id.clone()).collect();
        self.param_index = 0;
        for p in parameters {
            self.param_drafts.entry(p.id.clone()).or_insert_with(|| {
                p.default.clone().unwrap_or_else(|| match p.kind {
                    RecipeParameterKind::Boolean => "false".into(),
                    RecipeParameterKind::Choice => p.choices.first().cloned().unwrap_or_default(),
                    _ => String::new(),
                })
            });
        }
        self.preview = Some(format!("fill parameters for {title}"));
    }

    pub fn current_param(&self) -> Option<&RecipeParameter> {
        let id = self.param_order.get(self.param_index)?;
        let item = self.selected_item()?;
        match &item.kind {
            ShelfItemKind::RemoteCommand { parameters, .. } => {
                parameters.iter().find(|p| &p.id == id)
            }
            _ => None,
        }
    }

    pub fn param_next_field(&mut self) {
        if self.param_order.is_empty() {
            return;
        }
        self.param_index = (self.param_index + 1) % self.param_order.len();
        self.refresh_param_preview();
    }

    pub fn param_prev_field(&mut self) {
        if self.param_order.is_empty() {
            return;
        }
        if self.param_index == 0 {
            self.param_index = self.param_order.len() - 1;
        } else {
            self.param_index -= 1;
        }
        self.refresh_param_preview();
    }

    pub fn param_type_char(&mut self, c: char) {
        let Some(param) = self.current_param().cloned() else {
            return;
        };
        match param.kind {
            RecipeParameterKind::Boolean => {
                let cur = self
                    .param_drafts
                    .get(&param.id)
                    .map(|s| s.as_str())
                    .unwrap_or("false");
                let next = if matches!(c, ' ' | '\t' | 'y' | 'n' | 't' | 'f') {
                    if cur == "true" {
                        "false"
                    } else {
                        "true"
                    }
                } else {
                    return;
                };
                self.param_drafts.insert(param.id, next.into());
            }
            RecipeParameterKind::Choice => {
                if param.choices.is_empty() {
                    return;
                }
                let cur = self
                    .param_drafts
                    .get(&param.id)
                    .cloned()
                    .unwrap_or_default();
                let idx = param.choices.iter().position(|c| c == &cur).unwrap_or(0);
                let next = (idx + 1) % param.choices.len();
                self.param_drafts
                    .insert(param.id, param.choices[next].clone());
            }
            RecipeParameterKind::Integer => {
                if c.is_ascii_digit()
                    || (c == '-'
                        && self
                            .param_drafts
                            .get(&param.id)
                            .is_none_or(|s| s.is_empty()))
                {
                    self.param_drafts.entry(param.id).or_default().push(c);
                }
            }
            RecipeParameterKind::Text | RecipeParameterKind::Path => {
                if !c.is_control() {
                    let entry = self.param_drafts.entry(param.id.clone()).or_default();
                    if param
                        .max_length
                        .is_none_or(|max| entry.chars().count() < max)
                    {
                        entry.push(c);
                    }
                }
            }
        }
        self.refresh_param_preview();
    }

    pub fn param_backspace(&mut self) {
        let Some(param) = self.current_param().cloned() else {
            return;
        };
        if matches!(
            param.kind,
            RecipeParameterKind::Text | RecipeParameterKind::Path | RecipeParameterKind::Integer
        ) {
            if let Some(val) = self.param_drafts.get_mut(&param.id) {
                val.pop();
            }
            self.refresh_param_preview();
        }
    }

    fn refresh_param_preview(&mut self) {
        let Some(param) = self.current_param() else {
            return;
        };
        let value = self
            .param_drafts
            .get(&param.id)
            .cloned()
            .unwrap_or_default();
        self.preview = Some(format!(
            "{} ({}/{}) [{}]: {}",
            param.label,
            self.param_index + 1,
            self.param_order.len(),
            param.kind.as_str(),
            value
        ));
    }

    pub fn param_form_lines(&self) -> Vec<String> {
        let Some(item) = self.selected_item() else {
            return Vec::new();
        };
        let ShelfItemKind::RemoteCommand { parameters, .. } = &item.kind else {
            return Vec::new();
        };
        parameters
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let marker = if i == self.param_index { ">" } else { " " };
                let value = self.param_drafts.get(&p.id).cloned().unwrap_or_default();
                format!("{marker} {}: {}", p.label, value)
            })
            .collect()
    }

    pub fn rendered_command(&self, ssh: &SshRecipeContext) -> Option<String> {
        let item = self.selected_item()?;
        match &item.kind {
            ShelfItemKind::SshNative { id } => Some(native_text(id, ssh)),
            ShelfItemKind::RemoteCommand {
                program,
                args,
                parameters,
                ..
            } => {
                if self.filling_params && !self.params_ready(parameters) {
                    return None;
                }
                render_remote_command(program, args, parameters, &self.param_drafts, ssh).ok()
            }
        }
    }

    pub fn risk_of_selected(&self) -> Option<&str> {
        match &self.selected_item()?.kind {
            ShelfItemKind::RemoteCommand { risk, .. } => Some(risk.as_str()),
            _ => None,
        }
    }
}

fn native_text(id: &str, ssh: &SshRecipeContext) -> String {
    match id {
        "copy_alias" => ssh.alias.clone(),
        "copy_ip" => ssh.hostname.clone(),
        "copy_ssh" => format!("ssh -- {}", shell_quote(&ssh.alias)),
        "copy_sftp" => format!("sftp {}", shell_quote(&ssh.alias)),
        "show_info" => format!(
            "alias={} user={} host={} port={}",
            ssh.alias, ssh.user, ssh.hostname, ssh.port
        ),
        "reconnect" => "reconnect".into(),
        "disconnect" => "disconnect".into(),
        _ => id.into(),
    }
}

fn static_ssh_ops() -> Vec<ShelfItem> {
    [
        ("copy_alias", "Copy alias", "Copy host alias"),
        ("copy_ip", "Copy IP / hostname", "Copy resolved hostname"),
        ("copy_ssh", "Copy SSH command", "ssh user@host"),
        ("copy_sftp", "Copy SFTP command", "sftp user@host"),
        ("show_info", "Show host information", "Connection summary"),
        ("reconnect", "Reconnect", "Restart embedded session"),
        ("disconnect", "Disconnect", "Kill SSH child"),
    ]
    .into_iter()
    .map(|(id, title, description)| ShelfItem {
        group: "SSH".into(),
        title: title.into(),
        description: description.into(),
        favorite: false,
        use_count: 0,
        kind: ShelfItemKind::SshNative { id },
    })
    .collect()
}

fn static_fallback_commands() -> Vec<ShelfItem> {
    let mk = |group: &str, title: &str, program: &str, args: &[&str]| ShelfItem {
        group: group.into(),
        title: title.into(),
        description: String::new(),
        favorite: false,
        use_count: 0,
        kind: ShelfItemKind::RemoteCommand {
            recipe_id: None,
            program: program.into(),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            parameters: vec![],
            risk: "safe".into(),
        },
    };
    vec![
        mk("System", "uptime", "uptime", &[]),
        mk("System", "df -h", "df", &["-h"]),
        mk("System", "free -h", "free", &["-h"]),
        mk("Docker", "docker ps", "docker", &["ps"]),
    ]
}

/// Build a text parameter definition helper for tests / forms.
pub fn text_param(id: &str, label: &str, required: bool) -> RecipeParameter {
    RecipeParameter {
        id: id.into(),
        label: label.into(),
        description: String::new(),
        kind: RecipeParameterKind::Text,
        required,
        default: None,
        choices: vec![],
        min: None,
        max: None,
        pattern: None,
        max_length: Some(256),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_domain::{CommandStep, RecipeRisk, RecipeVariant};

    fn ssh_ctx() -> SshRecipeContext {
        SshRecipeContext {
            alias: "prod".into(),
            hostname: "1.2.3.4".into(),
            user: "root".into(),
            port: 22,
        }
    }

    fn step(program: &str, args: &[&str]) -> CommandStep {
        CommandStep {
            id: "1".into(),
            label: String::new(),
            program: program.into(),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            cwd: "current".into(),
            continue_on_error: false,
        }
    }

    fn variant(steps: Vec<CommandStep>) -> RecipeVariant {
        RecipeVariant {
            id: "default".into(),
            description: String::new(),
            requires_files: vec![],
            requires_directories: vec![],
            requires_commands: vec![],
            steps,
        }
    }

    #[test]
    fn insert_candidate_has_no_trailing_newline() {
        let mut shelf = ShelfState::from_recipes(&[], true);
        if let Some(idx) = shelf
            .items
            .iter()
            .position(|i| matches!(i.kind, ShelfItemKind::SshNative { id: "copy_ssh" }))
        {
            shelf.filtered = vec![idx];
            shelf.selected = 0;
        }
        let cmd = shelf.rendered_command(&ssh_ctx()).expect("cmd");
        assert!(!cmd.ends_with('\n') && !cmd.ends_with('\r'));
        assert!(cmd.contains("ssh"));
    }

    #[test]
    fn copied_connection_commands_preserve_the_ssh_config_alias() {
        let mut shelf = ShelfState::from_recipes(&[], true);
        let ssh_index = shelf
            .items
            .iter()
            .position(|item| matches!(item.kind, ShelfItemKind::SshNative { id: "copy_ssh" }))
            .expect("ssh item");
        shelf.filtered = vec![ssh_index];
        shelf.selected = 0;
        assert_eq!(
            shelf.rendered_command(&ssh_ctx()).as_deref(),
            Some("ssh -- prod")
        );

        let sftp_index = shelf
            .items
            .iter()
            .position(|item| matches!(item.kind, ShelfItemKind::SshNative { id: "copy_sftp" }))
            .expect("sftp item");
        shelf.filtered = vec![sftp_index];
        assert_eq!(
            shelf.rendered_command(&ssh_ctx()).as_deref(),
            Some("sftp prod")
        );
    }

    #[test]
    fn filter_narrows_selection() {
        let mut shelf = ShelfState::from_recipes(&[], true);
        shelf.filter = "sftp".into();
        shelf.refilter();
        assert_eq!(shelf.filtered.len(), 1);
        assert_eq!(shelf.selected_item().unwrap().title, "Copy SFTP command");
    }

    #[test]
    fn param_form_tab_and_typing() {
        let recipe = Recipe {
            id: "ssh-tail".into(),
            title: "tail".into(),
            description: String::new(),
            tags: vec![],
            scope: RecipeScope::SshSession,
            target: RecipeTarget::RemoteShell,
            group: "System".into(),
            risk: RecipeRisk::Safe,
            parameters: vec![text_param("path", "Path", true)],
            variants: vec![variant(vec![step("tail", &["-n", "50", "${path}"])])],
            enabled: true,
        };
        let mut shelf = ShelfState::from_recipes(&[recipe], false);
        shelf.selected = 0;
        let preview = shelf.begin_preview_or_params(&ssh_ctx());
        assert!(shelf.filling_params);
        assert!(preview.unwrap().contains("fill parameters"));
        shelf.param_type_char('/');
        shelf.param_type_char('v');
        shelf.param_type_char('a');
        shelf.param_type_char('r');
        shelf.param_next_field();
        assert_eq!(
            shelf.param_drafts.get("path").map(String::as_str),
            Some("/var")
        );
        let cmd = shelf.begin_preview_or_params(&ssh_ctx()).expect("rendered");
        assert!(cmd.contains("/var"));
        assert!(!cmd.ends_with('\n'));
    }

    #[test]
    fn favorites_sort_before_other_remotes() {
        let mk = |id: &str, title: &str| Recipe {
            id: id.into(),
            title: title.into(),
            description: String::new(),
            tags: vec![],
            scope: RecipeScope::SshSession,
            target: RecipeTarget::RemoteShell,
            group: "System".into(),
            risk: RecipeRisk::Safe,
            parameters: vec![],
            variants: vec![variant(vec![step("true", &[])])],
            enabled: true,
        };
        let mut meta = BTreeMap::new();
        meta.insert(
            "b".into(),
            RecipeMetadata {
                favorite: true,
                ..RecipeMetadata::default()
            },
        );
        let shelf =
            ShelfState::from_recipes_with_meta(&[mk("a", "aaa"), mk("b", "bbb")], &meta, false);
        assert_eq!(shelf.items[0].title, "bbb");
        assert!(shelf.items[0].favorite);
    }
}
