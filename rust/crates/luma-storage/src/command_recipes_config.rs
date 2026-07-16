use crate::command_recipes_builtin::builtin_recipes;
use luma_domain::{ConfigIssue, Recipe, RecipeCatalog, RecipeRisk, RecipeScope};
use serde::Deserialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CommandRecipesConfigError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml: {0}")]
    Toml(#[from] toml::de::Error),
}

#[derive(Debug, Deserialize)]
struct UserRecipesFile {
    #[serde(default)]
    recipes: Vec<UserRecipeToml>,
}

#[derive(Debug, Deserialize)]
struct UserRecipeToml {
    id: String,
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    risk: Option<String>,
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    variants: Vec<UserVariantToml>,
}

#[derive(Debug, Deserialize)]
struct UserVariantToml {
    id: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    requires_files: Vec<String>,
    #[serde(default)]
    requires_directories: Vec<String>,
    #[serde(default)]
    requires_commands: Vec<String>,
    #[serde(default)]
    steps: Vec<UserStepToml>,
}

#[derive(Debug, Deserialize)]
struct UserStepToml {
    id: String,
    label: String,
    program: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default = "default_cwd")]
    cwd: String,
    #[serde(default)]
    continue_on_error: bool,
}

fn default_cwd() -> String {
    "current".into()
}

fn program_basename(program: &str) -> &str {
    program.rsplit('/').next().unwrap_or(program)
}

fn is_shell_program(program: &str) -> bool {
    matches!(
        program_basename(program),
        "sh" | "bash" | "zsh" | "fish" | "dash" | "ksh" | "csh" | "tcsh"
    )
}

fn is_script_interpreter(program: &str) -> bool {
    matches!(
        program_basename(program),
        "python" | "python3" | "node" | "nodejs" | "ruby" | "perl" | "osascript" | "lua" | "php"
    )
}

fn has_execute_flag(args: &[String]) -> bool {
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-c" => {
                let allow_git_style = iter
                    .peek()
                    .is_some_and(|next| next.contains('=') && !next.starts_with('-'));
                if !allow_git_style {
                    return true;
                }
            }
            "-e" | "--eval" | "--exec" => return true,
            _ => {}
        }
    }
    false
}

fn is_script_path(program: &str) -> bool {
    let basename = program_basename(program);
    let lower = basename.to_ascii_lowercase();
    lower.ends_with(".sh")
        || lower.ends_with(".bash")
        || lower.ends_with(".zsh")
        || lower.ends_with(".py")
        || lower.ends_with(".rb")
        || lower.ends_with(".pl")
        || lower.ends_with(".lua")
        || lower.ends_with(".php")
        || lower.ends_with(".js")
        || lower.ends_with(".mjs")
        || lower.ends_with(".cjs")
        || lower.ends_with(".tcl")
        || lower.ends_with(".awk")
}

fn validate_user_step(step: &UserStepToml) -> Option<String> {
    if is_shell_program(&step.program) || is_script_interpreter(&step.program) {
        return Some(format!(
            "recipe step `{}`: shell/script interpreters are not allowed",
            step.id
        ));
    }
    if is_script_path(&step.program) {
        return Some(format!(
            "recipe step `{}`: script file paths are not allowed",
            step.id
        ));
    }
    if has_execute_flag(&step.args) {
        return Some(format!(
            "recipe step `{}`: execute flags are not allowed",
            step.id
        ));
    }
    if program_basename(&step.program) == "env" && !step.args.is_empty() {
        return Some(format!(
            "recipe step `{}`: env must have no args (use built-in show-env)",
            step.id
        ));
    }
    None
}

fn validate_user_recipe(user: &UserRecipeToml) -> Option<String> {
    for variant in &user.variants {
        for step in &variant.steps {
            if let Some(message) = validate_user_step(step) {
                return Some(format!("recipe `{}`: {message}", user.id));
            }
        }
    }
    None
}

pub fn command_recipes_config_path(support_dir: &Path) -> PathBuf {
    support_dir.join("command-recipes.toml")
}

pub fn load_recipe_catalog(config_path: &Path) -> RecipeCatalog {
    let builtins = builtin_recipes();
    let mut recipes: Vec<Recipe> = builtins.clone();
    let mut issues = Vec::new();

    if !config_path.exists() {
        return RecipeCatalog {
            recipes,
            issues,
            config_path: Some(config_path.to_path_buf()),
        };
    }

    let raw = match std::fs::read_to_string(config_path) {
        Ok(s) => s,
        Err(err) => {
            issues.push(ConfigIssue {
                location: config_path.display().to_string(),
                message: format!("cannot read config: {err}"),
                fatal: true,
            });
            return RecipeCatalog {
                recipes,
                issues,
                config_path: Some(config_path.to_path_buf()),
            };
        }
    };

    let parsed: UserRecipesFile = match toml::from_str(&raw) {
        Ok(f) => f,
        Err(err) => {
            issues.push(ConfigIssue {
                location: config_path.display().to_string(),
                message: format!("TOML parse error: {err}"),
                fatal: true,
            });
            return RecipeCatalog {
                recipes,
                issues,
                config_path: Some(config_path.to_path_buf()),
            };
        }
    };

    let mut seen_ids = HashSet::new();
    for user in parsed.recipes {
        if user.id.trim().is_empty() {
            issues.push(ConfigIssue {
                location: config_path.display().to_string(),
                message: "recipe missing id".into(),
                fatal: false,
            });
            continue;
        }
        if !seen_ids.insert(user.id.clone()) {
            issues.push(ConfigIssue {
                location: config_path.display().to_string(),
                message: format!("duplicate recipe id `{}`", user.id),
                fatal: false,
            });
            continue;
        }
        if user.title.trim().is_empty() {
            issues.push(ConfigIssue {
                location: config_path.display().to_string(),
                message: format!("recipe `{}` missing title", user.id),
                fatal: false,
            });
            continue;
        }
        if let Some(message) = validate_user_recipe(&user) {
            issues.push(ConfigIssue {
                location: config_path.display().to_string(),
                message,
                fatal: false,
            });
            continue;
        }
        let scope = match user.scope.as_deref() {
            Some(raw) => RecipeScope::parse(raw).unwrap_or_else(|| {
                issues.push(ConfigIssue {
                    location: config_path.display().to_string(),
                    message: format!("recipe `{}`: unknown scope `{raw}`", user.id),
                    fatal: false,
                });
                RecipeScope::CurrentProject
            }),
            None => RecipeScope::CurrentProject,
        };
        let risk = user
            .risk
            .as_deref()
            .and_then(RecipeRisk::parse)
            .unwrap_or(RecipeRisk::Confirm);
        let enabled = user.enabled.unwrap_or(true);
        let mut recipe = Recipe {
            id: user.id.clone(),
            title: user.title,
            description: user.description,
            tags: user.tags,
            scope,
            risk,
            enabled,
            variants: user
                .variants
                .into_iter()
                .map(|v| luma_domain::RecipeVariant {
                    id: v.id,
                    description: v.description,
                    requires_files: v.requires_files,
                    requires_directories: v.requires_directories,
                    requires_commands: v.requires_commands,
                    steps: v
                        .steps
                        .into_iter()
                        .map(|s| luma_domain::CommandStep {
                            id: s.id,
                            label: s.label,
                            program: s.program,
                            args: s.args,
                            cwd: s.cwd,
                            continue_on_error: s.continue_on_error,
                        })
                        .collect(),
                })
                .collect(),
        };
        if let Some(builtin) = builtins.iter().find(|builtin| builtin.id == recipe.id) {
            recipe.risk = RecipeRisk::max(recipe.risk.clone(), builtin.risk.clone());
        }
        if let Some(idx) = recipes.iter().position(|r| r.id == recipe.id) {
            recipes[idx] = recipe;
        } else {
            recipes.push(recipe);
        }
    }

    RecipeCatalog {
        recipes,
        issues,
        config_path: Some(config_path.to_path_buf()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn builtin_recipes_load_without_user_file() {
        let dir = tempdir().unwrap();
        let path = command_recipes_config_path(dir.path());
        let catalog = load_recipe_catalog(&path);
        assert!(catalog.issues.is_empty());
        assert!(catalog.recipe_by_id("git-status").is_some());
        assert!(catalog.recipe_by_id("luma-check").is_some());
    }

    #[test]
    fn user_recipe_overrides_builtin() {
        let dir = tempdir().unwrap();
        let path = command_recipes_config_path(dir.path());
        fs::write(
            &path,
            r#"
[[recipes]]
id = "git-status"
title = "Custom status"
description = "override"
risk = "safe"
scope = "current_project"

[[recipes.variants]]
id = "custom"
requires_files = [".git"]
requires_commands = ["git"]

[[recipes.variants.steps]]
id = "s1"
label = "custom status"
program = "git"
args = ["status"]
cwd = "current"
"#,
        )
        .unwrap();
        let catalog = load_recipe_catalog(&path);
        let recipe = catalog.recipe_by_id("git-status").unwrap();
        assert_eq!(recipe.title, "Custom status");
        assert_eq!(recipe.variants[0].id, "custom");
    }

    #[test]
    fn disabled_builtin_via_user_config() {
        let dir = tempdir().unwrap();
        let path = command_recipes_config_path(dir.path());
        fs::write(
            &path,
            r#"
[[recipes]]
id = "git-status"
title = "Git status"
enabled = false
variants = []
"#,
        )
        .unwrap();
        let catalog = load_recipe_catalog(&path);
        let recipe = catalog.recipe_by_id("git-status").unwrap();
        assert!(!recipe.enabled);
    }

    #[test]
    fn toml_syntax_error_is_non_fatal() {
        let dir = tempdir().unwrap();
        let path = command_recipes_config_path(dir.path());
        fs::write(&path, "[[recipes\nbroken").unwrap();
        let catalog = load_recipe_catalog(&path);
        assert!(!catalog.issues.is_empty());
        assert!(catalog.recipe_by_id("git-status").is_some());
    }

    #[test]
    fn unknown_risk_defaults_to_confirm() {
        let dir = tempdir().unwrap();
        let path = command_recipes_config_path(dir.path());
        fs::write(
            &path,
            r#"
[[recipes]]
id = "custom-cmd"
title = "Custom"
risk = "unknown-risk"
variants = []
"#,
        )
        .unwrap();
        let catalog = load_recipe_catalog(&path);
        let recipe = catalog.recipe_by_id("custom-cmd").unwrap();
        assert_eq!(recipe.risk, RecipeRisk::Confirm);
    }

    #[test]
    fn duplicate_id_is_warning_not_fatal() {
        let dir = tempdir().unwrap();
        let path = command_recipes_config_path(dir.path());
        fs::write(
            &path,
            r#"
[[recipes]]
id = "dup"
title = "One"
variants = []

[[recipes]]
id = "dup"
title = "Two"
variants = []
"#,
        )
        .unwrap();
        let catalog = load_recipe_catalog(&path);
        assert!(!catalog.has_fatal_issues());
        assert!(!catalog.issues.is_empty());
        assert!(catalog.recipe_by_id("git-status").is_some());
    }

    #[test]
    fn user_cannot_lower_builtin_risk() {
        let dir = tempdir().unwrap();
        let path = command_recipes_config_path(dir.path());
        fs::write(
            &path,
            r#"
[[recipes]]
id = "show-env"
title = "Show environment"
risk = "safe"
variants = []
"#,
        )
        .unwrap();
        let catalog = load_recipe_catalog(&path);
        let recipe = catalog.recipe_by_id("show-env").unwrap();
        assert_eq!(recipe.risk, RecipeRisk::Confirm);
    }

    #[test]
    fn shell_program_in_user_recipe_is_rejected() {
        let dir = tempdir().unwrap();
        let path = command_recipes_config_path(dir.path());
        fs::write(
            &path,
            r#"
[[recipes]]
id = "evil"
title = "Evil"

[[recipes.variants]]
id = "v1"

[[recipes.variants.steps]]
id = "s1"
label = "shell"
program = "sh"
args = ["-c", "rm -rf /"]
cwd = "current"
"#,
        )
        .unwrap();
        let catalog = load_recipe_catalog(&path);
        assert!(catalog.recipe_by_id("evil").is_none());
        assert!(catalog
            .issues
            .iter()
            .any(|issue| issue.message.contains("shell/script interpreters")));
    }

    #[test]
    fn git_style_c_flag_is_allowed() {
        let dir = tempdir().unwrap();
        let path = command_recipes_config_path(dir.path());
        fs::write(
            &path,
            r#"
[[recipes]]
id = "git-cfg"
title = "Git config"

[[recipes.variants]]
id = "v1"
requires_files = [".git"]
requires_commands = ["git"]

[[recipes.variants.steps]]
id = "s1"
label = "git -c"
program = "git"
args = ["-c", "user.email=me@example.com", "status"]
cwd = "current"
"#,
        )
        .unwrap();
        let catalog = load_recipe_catalog(&path);
        assert!(catalog.recipe_by_id("git-cfg").is_some());
    }

    #[test]
    fn script_file_path_is_rejected() {
        let dir = tempdir().unwrap();
        let path = command_recipes_config_path(dir.path());
        fs::write(
            &path,
            r#"
[[recipes]]
id = "evil"
title = "Evil"

[[recipes.variants]]
id = "v1"

[[recipes.variants.steps]]
id = "s1"
label = "script"
program = "./scripts/deploy.sh"
args = []
cwd = "current"
"#,
        )
        .unwrap();
        let catalog = load_recipe_catalog(&path);
        assert!(catalog.recipe_by_id("evil").is_none());
        assert!(catalog
            .issues
            .iter()
            .any(|issue| issue.message.contains("script file paths")));
    }
}
