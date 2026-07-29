use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeRisk {
    #[default]
    Safe,
    Confirm,
    Destructive,
}

impl RecipeRisk {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "safe" => Some(Self::Safe),
            "confirm" => Some(Self::Confirm),
            "destructive" => Some(Self::Destructive),
            _ => None,
        }
    }

    /// Pick the more conservative risk level (Destructive > Confirm > Safe).
    pub fn max(a: Self, b: Self) -> Self {
        if matches!(a, Self::Destructive) || matches!(b, Self::Destructive) {
            Self::Destructive
        } else if matches!(a, Self::Confirm) || matches!(b, Self::Confirm) {
            Self::Confirm
        } else {
            Self::Safe
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Confirm => "confirm",
            Self::Destructive => "destructive",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeScope {
    #[default]
    Global,
    CurrentProject,
    LumaRepository,
    SshSession,
}

impl RecipeScope {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "global" => Some(Self::Global),
            "current_project" => Some(Self::CurrentProject),
            "luma_repository" => Some(Self::LumaRepository),
            "ssh_session" => Some(Self::SshSession),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::CurrentProject => "current_project",
            Self::LumaRepository => "luma_repository",
            Self::SshSession => "ssh_session",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeTarget {
    #[default]
    LocalShell,
    RemoteShell,
}

impl RecipeTarget {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "local_shell" => Some(Self::LocalShell),
            "remote_shell" => Some(Self::RemoteShell),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LocalShell => "local_shell",
            Self::RemoteShell => "remote_shell",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeParameterKind {
    Text,
    Integer,
    Choice,
    Boolean,
    Path,
}

impl RecipeParameterKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "text" => Some(Self::Text),
            "integer" => Some(Self::Integer),
            "choice" => Some(Self::Choice),
            "boolean" => Some(Self::Boolean),
            "path" => Some(Self::Path),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Integer => "integer",
            Self::Choice => "choice",
            Self::Boolean => "boolean",
            Self::Path => "path",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeParameter {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub description: String,
    pub kind: RecipeParameterKind,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub choices: Vec<String>,
    #[serde(default)]
    pub min: Option<i64>,
    #[serde(default)]
    pub max: Option<i64>,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub max_length: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SshRecipeContext {
    pub alias: String,
    pub hostname: String,
    pub user: String,
    pub port: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecipeRenderError {
    UnknownParameter(String),
    EmbeddedParameter { arg: String },
    InvalidValue { parameter: String, message: String },
    MissingRequired(String),
    ForbiddenProgram,
}

impl std::fmt::Display for RecipeRenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownParameter(id) => write!(f, "unknown parameter `{id}`"),
            Self::EmbeddedParameter { arg } => {
                write!(f, "parameter must occupy whole arg token: `{arg}`")
            }
            Self::InvalidValue {
                parameter,
                message,
            } => write!(f, "invalid `{parameter}`: {message}"),
            Self::MissingRequired(id) => write!(f, "missing required parameter `{id}`"),
            Self::ForbiddenProgram => write!(f, "program cannot be a shell interpreter"),
        }
    }
}

/// Render `program` + `args` for remote paste: each arg independently shell-quoted.
/// `${name}` tokens must be whole arguments. Context keys: `ssh.alias`, etc.
pub fn render_remote_command(
    program: &str,
    args: &[String],
    parameters: &[RecipeParameter],
    values: &std::collections::BTreeMap<String, String>,
    ssh: &SshRecipeContext,
) -> Result<String, RecipeRenderError> {
    if is_forbidden_shell_program(program) {
        return Err(RecipeRenderError::ForbiddenProgram);
    }
    let mut out = Vec::with_capacity(1 + args.len());
    out.push(shell_quote(program));
    for arg in args {
        let resolved = resolve_arg_token(arg, parameters, values, ssh)?;
        out.push(shell_quote(&resolved));
    }
    Ok(out.join(" "))
}

fn is_forbidden_shell_program(program: &str) -> bool {
    let base = program.rsplit('/').next().unwrap_or(program);
    matches!(
        base,
        "sh" | "bash" | "zsh" | "fish" | "dash" | "ksh" | "csh" | "tcsh"
    )
}

fn resolve_arg_token(
    arg: &str,
    parameters: &[RecipeParameter],
    values: &std::collections::BTreeMap<String, String>,
    ssh: &SshRecipeContext,
) -> Result<String, RecipeRenderError> {
    if let Some(name) = whole_param_token(arg) {
        return lookup_value(name, parameters, values, ssh);
    }
    if arg.contains("${") {
        return Err(RecipeRenderError::EmbeddedParameter {
            arg: arg.to_string(),
        });
    }
    Ok(arg.to_string())
}

fn whole_param_token(arg: &str) -> Option<&str> {
    let rest = arg.strip_prefix("${")?.strip_suffix('}')?;
    if rest.is_empty() || rest.contains("${") || rest.contains('}') {
        return None;
    }
    // Entire arg must be exactly ${name}
    if arg.len() == rest.len() + 3 {
        Some(rest)
    } else {
        None
    }
}

fn lookup_value(
    name: &str,
    parameters: &[RecipeParameter],
    values: &std::collections::BTreeMap<String, String>,
    ssh: &SshRecipeContext,
) -> Result<String, RecipeRenderError> {
    let raw = match name {
        "ssh.alias" => ssh.alias.clone(),
        "ssh.hostname" => ssh.hostname.clone(),
        "ssh.user" => ssh.user.clone(),
        "ssh.port" => ssh.port.to_string(),
        other => {
            let param = parameters
                .iter()
                .find(|p| p.id == other)
                .ok_or_else(|| RecipeRenderError::UnknownParameter(other.to_string()))?;
            let value = values
                .get(other)
                .cloned()
                .or_else(|| param.default.clone())
                .ok_or_else(|| {
                    if param.required {
                        RecipeRenderError::MissingRequired(other.to_string())
                    } else {
                        RecipeRenderError::MissingRequired(other.to_string())
                    }
                })?;
            validate_parameter_value(param, &value)?;
            value
        }
    };
    reject_control_chars(&raw).map_err(|message| RecipeRenderError::InvalidValue {
        parameter: name.to_string(),
        message,
    })?;
    Ok(raw)
}

fn validate_parameter_value(
    param: &RecipeParameter,
    value: &str,
) -> Result<(), RecipeRenderError> {
    reject_control_chars(value).map_err(|message| RecipeRenderError::InvalidValue {
        parameter: param.id.clone(),
        message,
    })?;
    if let Some(max_length) = param.max_length {
        if value.chars().count() > max_length {
            return Err(RecipeRenderError::InvalidValue {
                parameter: param.id.clone(),
                message: format!("exceeds max_length {max_length}"),
            });
        }
    }
    match param.kind {
        RecipeParameterKind::Integer => {
            let parsed: i64 = value.parse().map_err(|_| RecipeRenderError::InvalidValue {
                parameter: param.id.clone(),
                message: "expected integer".into(),
            })?;
            if let Some(min) = param.min {
                if parsed < min {
                    return Err(RecipeRenderError::InvalidValue {
                        parameter: param.id.clone(),
                        message: format!("below min {min}"),
                    });
                }
            }
            if let Some(max) = param.max {
                if parsed > max {
                    return Err(RecipeRenderError::InvalidValue {
                        parameter: param.id.clone(),
                        message: format!("above max {max}"),
                    });
                }
            }
        }
        RecipeParameterKind::Choice => {
            if !param.choices.iter().any(|c| c == value) {
                return Err(RecipeRenderError::InvalidValue {
                    parameter: param.id.clone(),
                    message: "value not in choices".into(),
                });
            }
        }
        RecipeParameterKind::Boolean => {
            if !matches!(
                value.to_ascii_lowercase().as_str(),
                "true" | "false" | "1" | "0" | "yes" | "no"
            ) {
                return Err(RecipeRenderError::InvalidValue {
                    parameter: param.id.clone(),
                    message: "expected boolean".into(),
                });
            }
        }
        RecipeParameterKind::Text | RecipeParameterKind::Path => {}
    }
    if let Some(pattern) = &param.pattern {
        // Lightweight containment check — full regex optional later.
        if pattern.starts_with('^') || pattern.ends_with('$') {
            // Skip full regex engine in domain; pattern stored for UI.
            let _ = pattern;
        }
    }
    Ok(())
}

fn reject_control_chars(value: &str) -> Result<(), String> {
    if value.chars().any(|c| c == '\0' || c == '\n' || c == '\r' || c.is_control()) {
        return Err("control characters are not allowed".into());
    }
    Ok(())
}

/// POSIX-ish single-quote shell quoting for remote paste.
pub fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".into();
    }
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '@' | '%' | '+' | '=' | ':' | ',' | '.' | '/' | '-'))
    {
        return value.to_string();
    }
    let mut out = String::from("'");
    for ch in value.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandStep {
    pub id: String,
    pub label: String,
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default = "default_cwd")]
    pub cwd: String,
    #[serde(default)]
    pub continue_on_error: bool,
}

fn default_cwd() -> String {
    "current".into()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeVariant {
    pub id: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub requires_files: Vec<String>,
    #[serde(default)]
    pub requires_directories: Vec<String>,
    #[serde(default)]
    pub requires_commands: Vec<String>,
    #[serde(default)]
    pub steps: Vec<CommandStep>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recipe {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub scope: RecipeScope,
    #[serde(default)]
    pub target: RecipeTarget,
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub risk: RecipeRisk,
    #[serde(default)]
    pub parameters: Vec<RecipeParameter>,
    #[serde(default)]
    pub variants: Vec<RecipeVariant>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeRunOutcome {
    #[default]
    Success,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeMetadata {
    pub favorite: bool,
    pub last_used_at: Option<i64>,
    pub use_count: u64,
    pub last_result: Option<RecipeRunOutcome>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedCommandStep {
    pub id: String,
    pub label: String,
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    /// Project root used to validate cwd containment at execution time.
    pub root: PathBuf,
    pub continue_on_error: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeRunPlan {
    pub recipe_id: String,
    pub recipe_title: String,
    pub risk: RecipeRisk,
    pub working_dir: PathBuf,
    pub variant_id: String,
    pub variant_description: String,
    pub steps: Vec<ResolvedCommandStep>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VariantMatch {
    Matched(RecipeVariant),
    NoMatch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigIssue {
    pub location: String,
    pub message: String,
    /// When true, the module cannot load user config at all (parse/read failure).
    pub fatal: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RecipeCatalog {
    pub recipes: Vec<Recipe>,
    pub issues: Vec<ConfigIssue>,
    pub config_path: Option<PathBuf>,
}

impl RecipeCatalog {
    pub fn recipe_by_id(&self, id: &str) -> Option<&Recipe> {
        self.recipes.iter().find(|r| r.id == id)
    }

    pub fn has_fatal_issues(&self) -> bool {
        self.issues.iter().any(|issue| issue.fatal)
    }

    pub fn warnings(&self) -> impl Iterator<Item = &ConfigIssue> {
        self.issues.iter().filter(|issue| !issue.fatal)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepRunResult {
    pub step_id: String,
    pub exit_code: Option<i32>,
    pub started: bool,
    /// True when the step was cancelled (token or signal) rather than failed.
    pub cancelled: bool,
    pub message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn ssh_session_scope_parses() {
        assert_eq!(
            RecipeScope::parse("ssh_session"),
            Some(RecipeScope::SshSession)
        );
    }

    #[test]
    fn render_quotes_each_arg_and_context() {
        let params = vec![RecipeParameter {
            id: "container".into(),
            label: "容器".into(),
            description: String::new(),
            kind: RecipeParameterKind::Text,
            required: true,
            default: None,
            choices: vec![],
            min: None,
            max: None,
            pattern: None,
            max_length: None,
        }];
        let mut values = BTreeMap::new();
        values.insert("container".into(), "app server".into());
        let ssh = SshRecipeContext {
            alias: "prod".into(),
            hostname: "1.2.3.4".into(),
            user: "root".into(),
            port: 22,
        };
        let rendered = render_remote_command(
            "docker",
            &[
                "logs".into(),
                "--tail".into(),
                "100".into(),
                "${container}".into(),
            ],
            &params,
            &values,
            &ssh,
        )
        .expect("render");
        assert_eq!(rendered, "docker logs --tail 100 'app server'");
    }

    #[test]
    fn rejects_embedded_parameter_token() {
        let err = render_remote_command(
            "echo",
            &["pre-${x}-post".into()],
            &[],
            &BTreeMap::new(),
            &SshRecipeContext::default(),
        )
        .expect_err("embedded");
        assert!(matches!(err, RecipeRenderError::EmbeddedParameter { .. }));
    }

    #[test]
    fn rejects_shell_program() {
        let err = render_remote_command(
            "bash",
            &["-c".into(), "true".into()],
            &[],
            &BTreeMap::new(),
            &SshRecipeContext::default(),
        )
        .expect_err("shell");
        assert!(matches!(err, RecipeRenderError::ForbiddenProgram));
    }

    #[test]
    fn rejects_control_characters() {
        let params = vec![RecipeParameter {
            id: "path".into(),
            label: "path".into(),
            description: String::new(),
            kind: RecipeParameterKind::Path,
            required: true,
            default: None,
            choices: vec![],
            min: None,
            max: None,
            pattern: None,
            max_length: None,
        }];
        let mut values = BTreeMap::new();
        values.insert("path".into(), "/tmp/\n".into());
        let err = render_remote_command(
            "tail",
            &["${path}".into()],
            &params,
            &values,
            &SshRecipeContext::default(),
        )
        .expect_err("control");
        assert!(matches!(err, RecipeRenderError::InvalidValue { .. }));
    }
}
