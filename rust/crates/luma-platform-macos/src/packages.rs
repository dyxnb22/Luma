//! Homebrew-only package adapter with bounded, cancellable direct subprocess execution.

use async_trait::async_trait;
use luma_application::{
    mutation_args, validate_mutation_state, PackageError, PackageKind, PackageManagerPort,
    PackageMutation, PackageMutationPlan, PackageQuery, PackageRecord, MAX_PACKAGE_OUTPUT_BYTES,
    MAX_PACKAGE_RESULTS,
};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio_util::sync::CancellationToken;

const BREW_TIMEOUT: Duration = Duration::from_secs(12);
const MAX_PACKAGE_DESCRIPTION_BYTES: usize = 2 * 1024;
const MAX_PACKAGE_HOMEPAGE_BYTES: usize = 4 * 1024;
const MAX_PACKAGE_VERSION_BYTES: usize = 512;

pub struct MacHomebrew {
    program: Option<PathBuf>,
    timeout: Duration,
}

impl MacHomebrew {
    pub fn system_default() -> Self {
        Self {
            program: resolve_brew(),
            timeout: BREW_TIMEOUT,
        }
    }

    #[cfg(test)]
    fn with_program(program: PathBuf) -> Self {
        Self {
            program: Some(program),
            timeout: BREW_TIMEOUT,
        }
    }

    fn program(&self) -> Result<&Path, PackageError> {
        let program = self.program.as_deref().ok_or(PackageError::NotConfigured)?;
        if is_executable_file(program) {
            Ok(program)
        } else {
            Err(PackageError::NotConfigured)
        }
    }

    async fn run(
        &self,
        args: &[String],
        cancel: &CancellationToken,
    ) -> Result<Vec<u8>, PackageError> {
        if cancel.is_cancelled() {
            return Err(PackageError::Cancelled);
        }
        let program = self.program()?.to_path_buf();
        let mut child = tokio::process::Command::new(program)
            .args(args)
            // A targeted read must not unexpectedly update taps or download Homebrew runtime
            // components. Mutations are returned as an interactive terminal plan and therefore
            // do not pass through this query runner.
            .env("HOMEBREW_NO_AUTO_UPDATE", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|error| PackageError::Unavailable(error.to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| PackageError::Unavailable("stdout pipe unavailable".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| PackageError::Unavailable("stderr pipe unavailable".into()))?;
        let stdout_task = tokio::spawn(read_bounded(stdout));
        let stderr_task = tokio::spawn(read_bounded(stderr));
        let status = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                let _ = child.kill().await;
                stdout_task.abort();
                stderr_task.abort();
                return Err(PackageError::Cancelled);
            }
            _ = tokio::time::sleep(self.timeout) => {
                let _ = child.kill().await;
                stdout_task.abort();
                stderr_task.abort();
                return Err(PackageError::Timeout);
            }
            status = child.wait() => status.map_err(|error| PackageError::Unavailable(error.to_string()))?,
        };
        let stdout = stdout_task
            .await
            .map_err(|error| PackageError::Unavailable(error.to_string()))??;
        let stderr = stderr_task
            .await
            .map_err(|error| PackageError::Unavailable(error.to_string()))??;
        if stdout.len().saturating_add(stderr.len()) > MAX_PACKAGE_OUTPUT_BYTES {
            return Err(PackageError::OutputTooLarge(MAX_PACKAGE_OUTPUT_BYTES));
        }
        if !status.success() {
            return Err(PackageError::CommandFailed(sanitized_stderr(&stderr)));
        }
        Ok(stdout)
    }

    async fn json_query(
        &self,
        args: &[String],
        cancel: &CancellationToken,
    ) -> Result<Vec<PackageRecord>, PackageError> {
        let output = self.run(args, cancel).await?;
        parse_brew_json(&output)
    }

    async fn line_query(
        &self,
        args: &[String],
        kind: PackageKind,
        cancel: &CancellationToken,
    ) -> Result<Vec<PackageRecord>, PackageError> {
        let output = self.run(args, cancel).await?;
        parse_name_lines(&output, kind)
    }

    async fn search_lines(
        &self,
        args: &[String],
        kind: PackageKind,
        cancel: &CancellationToken,
    ) -> Result<Vec<PackageRecord>, PackageError> {
        match self.line_query(args, kind, cancel).await {
            Err(PackageError::CommandFailed(message)) if is_empty_brew_search(&message) => {
                Ok(Vec::new())
            }
            result => result,
        }
    }
}

#[async_trait]
impl PackageManagerPort for MacHomebrew {
    async fn query(
        &self,
        query: PackageQuery,
        limit: usize,
        cancel: CancellationToken,
    ) -> Result<Vec<PackageRecord>, PackageError> {
        let mut records = match query {
            PackageQuery::Installed => {
                self.json_query(&string_args(&["info", "--json=v2", "--installed"]), &cancel)
                    .await?
            }
            PackageQuery::Outdated => {
                self.json_query(&string_args(&["outdated", "--json=v2"]), &cancel)
                    .await?
            }
            PackageQuery::Formulae => {
                self.line_query(&string_args(&["formulae"]), PackageKind::Formula, &cancel)
                    .await?
            }
            PackageQuery::Casks => {
                self.line_query(&string_args(&["casks"]), PackageKind::Cask, &cancel)
                    .await?
            }
            PackageQuery::Search(needle) => {
                validate_search(&needle)?;
                let mut formulae = self
                    .search_lines(
                        &["search".into(), "--formula".into(), needle.clone()],
                        PackageKind::Formula,
                        &cancel,
                    )
                    .await?;
                if cancel.is_cancelled() {
                    return Err(PackageError::Cancelled);
                }
                let casks = self
                    .search_lines(
                        &["search".into(), "--cask".into(), needle],
                        PackageKind::Cask,
                        &cancel,
                    )
                    .await?;
                formulae.extend(casks);
                formulae
            }
            PackageQuery::Info(name) => {
                validate_package_name(&name)?;
                self.json_query(&["info".into(), "--json=v2".into(), name], &cancel)
                    .await?
            }
        };
        records.sort_by(|a, b| {
            a.name
                .cmp(&b.name)
                .then_with(|| a.kind.label().cmp(b.kind.label()))
        });
        records.dedup_by(|a, b| a.name == b.name && a.kind == b.kind);
        records.truncate(limit.min(MAX_PACKAGE_RESULTS));
        Ok(records)
    }

    async fn resolve(
        &self,
        name: &str,
        kind: PackageKind,
        cancel: CancellationToken,
    ) -> Result<PackageRecord, PackageError> {
        validate_package_name(name)?;
        let matches = self
            .query(PackageQuery::Info(name.into()), MAX_PACKAGE_RESULTS, cancel)
            .await?
            .into_iter()
            .filter(|record| record.name == name && record.kind == kind)
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [record] => Ok(record.clone()),
            [] => Err(PackageError::NotFound),
            _ => Err(PackageError::Ambiguous),
        }
    }

    async fn mutation_plan(
        &self,
        mutation: PackageMutation,
        name: &str,
        kind: PackageKind,
        cancel: CancellationToken,
    ) -> Result<PackageMutationPlan, PackageError> {
        if cancel.is_cancelled() {
            return Err(PackageError::Cancelled);
        }
        let package = self.resolve(name, kind, cancel.clone()).await?;
        if cancel.is_cancelled() {
            return Err(PackageError::Cancelled);
        }
        validate_mutation_state(mutation, &package)?;
        Ok(PackageMutationPlan {
            program: self
                .program()?
                .to_str()
                .ok_or_else(|| PackageError::Unavailable("brew path is not UTF-8".into()))?
                .into(),
            args: mutation_args(mutation, name, kind),
            package,
        })
    }
}

async fn read_bounded(reader: impl tokio::io::AsyncRead + Unpin) -> Result<Vec<u8>, PackageError> {
    let mut bytes = Vec::new();
    reader
        .take((MAX_PACKAGE_OUTPUT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .await
        .map_err(|error| PackageError::Unavailable(error.to_string()))?;
    if bytes.len() > MAX_PACKAGE_OUTPUT_BYTES {
        return Err(PackageError::OutputTooLarge(MAX_PACKAGE_OUTPUT_BYTES));
    }
    Ok(bytes)
}

fn string_args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).into()).collect()
}

fn parse_brew_json(bytes: &[u8]) -> Result<Vec<PackageRecord>, PackageError> {
    let root: Value = serde_json::from_slice(bytes)
        .map_err(|error| PackageError::Malformed(error.to_string()))?;
    let root = root
        .as_object()
        .ok_or_else(|| PackageError::Malformed("Homebrew JSON root must be an object".into()))?;
    if !root.contains_key("formulae") && !root.contains_key("casks") {
        return Err(PackageError::Malformed(
            "Homebrew JSON has neither `formulae` nor `casks`".into(),
        ));
    }
    let mut records = Vec::new();
    if let Some(formulae) = optional_array(root.get("formulae"), "formulae")? {
        for formula in formulae {
            if records.len() >= MAX_PACKAGE_RESULTS {
                break;
            }
            let name = package_identity(formula, "name")?;
            validate_package_name(&name)?;
            records.push(PackageRecord {
                name,
                kind: PackageKind::Formula,
                description: optional_string(formula, "desc", MAX_PACKAGE_DESCRIPTION_BYTES),
                homepage: optional_string(formula, "homepage", MAX_PACKAGE_HOMEPAGE_BYTES),
                version: formula
                    .get("versions")
                    .and_then(|versions| versions.get("stable"))
                    .and_then(Value::as_str)
                    .or_else(|| formula.get("current_version").and_then(Value::as_str))
                    .map(|value| bounded_single_line(value, MAX_PACKAGE_VERSION_BYTES)),
                installed: package_is_installed(formula),
                outdated: package_is_outdated(formula),
            });
        }
    }
    if let Some(casks) = optional_array(root.get("casks"), "casks")? {
        for cask in casks {
            if records.len() >= MAX_PACKAGE_RESULTS {
                break;
            }
            let name = cask_identity(cask)?;
            validate_package_name(&name)?;
            records.push(PackageRecord {
                name,
                kind: PackageKind::Cask,
                description: optional_string(cask, "desc", MAX_PACKAGE_DESCRIPTION_BYTES),
                homepage: optional_string(cask, "homepage", MAX_PACKAGE_HOMEPAGE_BYTES),
                version: optional_string(cask, "version", MAX_PACKAGE_VERSION_BYTES).or_else(
                    || optional_string(cask, "current_version", MAX_PACKAGE_VERSION_BYTES),
                ),
                installed: package_is_installed(cask),
                outdated: package_is_outdated(cask),
            });
        }
    }
    Ok(records)
}

fn optional_array<'a>(
    value: Option<&'a Value>,
    field: &str,
) -> Result<Option<&'a Vec<Value>>, PackageError> {
    value
        .map(|value| {
            value.as_array().ok_or_else(|| {
                PackageError::Malformed(format!("Homebrew field `{field}` must be an array"))
            })
        })
        .transpose()
}

fn required_string(value: &Value, field: &str) -> Result<String, PackageError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| PackageError::Malformed(format!("missing required field `{field}`")))
}

fn package_identity(value: &Value, fallback_field: &str) -> Result<String, PackageError> {
    value
        .get("full_name")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .map(Ok)
        .unwrap_or_else(|| required_string(value, fallback_field))
}

fn cask_identity(value: &Value) -> Result<String, PackageError> {
    if let Some(full_name) = value
        .get("full_name")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    {
        return Ok(full_name.into());
    }
    value
        .get("token")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .or_else(|| value.get("name").and_then(Value::as_str))
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| PackageError::Malformed("missing required cask identity".into()))
}

fn package_is_installed(value: &Value) -> bool {
    ["installed", "installed_versions"]
        .into_iter()
        .any(|field| {
            value.get(field).is_some_and(|installed| {
                installed.as_str().is_some_and(|value| !value.is_empty())
                    || installed
                        .as_array()
                        .is_some_and(|values| !values.is_empty())
            })
        })
}

fn package_is_outdated(value: &Value) -> bool {
    value
        .get("outdated")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || value
            .get("installed_versions")
            .and_then(Value::as_array)
            .is_some_and(|versions| !versions.is_empty())
}

fn optional_string(value: &Value, field: &str, max_bytes: usize) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(|value| bounded_single_line(value, max_bytes))
        .filter(|value| !value.is_empty())
}

fn bounded_single_line(value: &str, max_bytes: usize) -> String {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.len() <= max_bytes {
        return compact;
    }
    let mut end = max_bytes;
    while end > 0 && !compact.is_char_boundary(end) {
        end -= 1;
    }
    compact[..end].into()
}

fn parse_name_lines(bytes: &[u8], kind: PackageKind) -> Result<Vec<PackageRecord>, PackageError> {
    let text =
        std::str::from_utf8(bytes).map_err(|error| PackageError::Malformed(error.to_string()))?;
    let mut records = Vec::new();
    for line in text.lines() {
        let name = line.trim();
        if name.is_empty() || name.starts_with("==>") {
            continue;
        }
        validate_package_name(name)?;
        records.push(PackageRecord {
            name: name.into(),
            kind,
            description: None,
            homepage: None,
            version: None,
            installed: false,
            outdated: false,
        });
        if records.len() >= MAX_PACKAGE_RESULTS {
            break;
        }
    }
    Ok(records)
}

fn validate_search(query: &str) -> Result<(), PackageError> {
    if query.is_empty()
        || query.len() > 200
        || query.contains('\0')
        || query.starts_with('-')
        || query.contains(char::is_whitespace)
    {
        return Err(PackageError::Malformed(
            "search must be one non-option package token up to 200 bytes".into(),
        ));
    }
    Ok(())
}

fn validate_package_name(name: &str) -> Result<(), PackageError> {
    if name.is_empty()
        || name.len() > 200
        || name.starts_with(['-', '/'])
        || name.contains("..")
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"@+._-/".contains(&byte))
    {
        return Err(PackageError::Malformed("invalid package identity".into()));
    }
    Ok(())
}

fn sanitized_stderr(bytes: &[u8]) -> String {
    let value = String::from_utf8_lossy(bytes);
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.is_empty() {
        "nonzero exit".into()
    } else {
        compact.chars().take(512).collect()
    }
}

fn is_empty_brew_search(message: &str) -> bool {
    message.starts_with("Error: No formulae or casks found for \"") && message.ends_with("\".")
}

fn resolve_brew() -> Option<PathBuf> {
    let known = ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"];
    for path in known.into_iter().map(PathBuf::from) {
        if is_executable_file(&path) {
            return Some(path);
        }
    }
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .map(|directory| directory.join("brew"))
        .find(|path| is_executable_file(path))
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        metadata.is_file()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"{
      "formulae": [{
        "name": "ripgrep",
        "full_name": "example/tools/ripgrep",
        "desc": "Search tool",
        "homepage": "https://example.test/rg",
        "versions": {"stable": "14.1.0"},
        "installed": [{"version": "14.0.0"}],
        "outdated": true,
        "future_field": {"ignored": true}
      }],
      "casks": [{
        "token": "visual-studio-code",
        "full_name": "example/apps/visual-studio-code",
        "name": ["Visual Studio Code"],
        "desc": "Editor",
        "homepage": "https://example.test/code",
        "version": "1.2.3",
        "installed": null,
        "future": 42
      }]
    }"#;

    #[test]
    fn parses_formulae_casks_and_forward_fields() {
        let records = parse_brew_json(FIXTURE.as_bytes()).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].name, "example/tools/ripgrep");
        assert!(records[0].installed);
        assert!(records[0].outdated);
        assert_eq!(records[1].kind, PackageKind::Cask);
        assert_eq!(records[1].name, "example/apps/visual-studio-code");
        assert!(!records[1].installed);
    }

    #[test]
    fn malformed_json_and_missing_identity_are_rejected() {
        assert!(matches!(
            parse_brew_json(b"{"),
            Err(PackageError::Malformed(_))
        ));
        assert!(matches!(
            parse_brew_json(br#"{"formulae":[{"desc":"missing"}]}"#),
            Err(PackageError::Malformed(_))
        ));
        for input in [
            br#"{}"#.as_slice(),
            br#"{"formulae":{}}"#.as_slice(),
            br#"{"formulae":[{"name":"--hostile"}]}"#.as_slice(),
        ] {
            assert!(matches!(
                parse_brew_json(input),
                Err(PackageError::Malformed(_))
            ));
        }
    }

    #[test]
    fn parses_homebrew_six_outdated_schema_for_formulae_and_casks() {
        let records = parse_brew_json(
            br#"{
              "formulae": [{
                "name": "example/tools/probe",
                "installed_versions": ["1.0.0"],
                "current_version": "1.1.0"
              }],
              "casks": [{
                "name": "example/apps/probe-app",
                "installed_versions": ["2.0.0"],
                "current_version": "2.1.0"
              }]
            }"#,
        )
        .unwrap();

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].name, "example/tools/probe");
        assert_eq!(records[0].version.as_deref(), Some("1.1.0"));
        assert!(records[0].installed);
        assert!(records[0].outdated);
        assert_eq!(records[1].name, "example/apps/probe-app");
        assert_eq!(records[1].kind, PackageKind::Cask);
        assert_eq!(records[1].version.as_deref(), Some("2.1.0"));
        assert!(records[1].installed);
        assert!(records[1].outdated);
    }

    #[test]
    fn line_parser_is_bounded_and_validates_names() {
        let names = (0..600)
            .map(|index| format!("pkg-{index}"))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            parse_name_lines(names.as_bytes(), PackageKind::Formula)
                .unwrap()
                .len(),
            MAX_PACKAGE_RESULTS
        );
        assert!(parse_name_lines(b"--hostile\n", PackageKind::Formula).is_err());
    }

    #[tokio::test]
    async fn pre_cancel_does_not_spawn_and_missing_binary_is_not_configured() {
        let adapter = MacHomebrew::with_program(PathBuf::from("/missing/brew"));
        let cancel = CancellationToken::new();
        cancel.cancel();
        assert_eq!(
            adapter.query(PackageQuery::Installed, 10, cancel).await,
            Err(PackageError::Cancelled)
        );
        assert_eq!(
            adapter
                .query(PackageQuery::Installed, 10, CancellationToken::new())
                .await,
            Err(PackageError::NotConfigured)
        );
    }

    #[tokio::test]
    async fn timeout_nonzero_oversize_and_inflight_cancel_are_distinct() {
        let timeout_adapter = MacHomebrew {
            program: Some(PathBuf::from("/bin/sleep")),
            timeout: Duration::from_millis(10),
        };
        assert_eq!(
            timeout_adapter
                .run(&["30".into()], &CancellationToken::new())
                .await,
            Err(PackageError::Timeout)
        );

        let false_adapter = MacHomebrew::with_program(PathBuf::from("/usr/bin/false"));
        assert!(matches!(
            false_adapter.run(&[], &CancellationToken::new()).await,
            Err(PackageError::CommandFailed(_))
        ));

        let temp = tempfile::tempdir().unwrap();
        let oversized = temp.path().join("oversized");
        std::fs::write(&oversized, vec![b'x'; MAX_PACKAGE_OUTPUT_BYTES + 1]).unwrap();
        let cat_adapter = MacHomebrew::with_program(PathBuf::from("/bin/cat"));
        assert_eq!(
            cat_adapter
                .run(
                    &[oversized.to_string_lossy().into_owned()],
                    &CancellationToken::new()
                )
                .await,
            Err(PackageError::OutputTooLarge(MAX_PACKAGE_OUTPUT_BYTES))
        );

        let cancel_adapter = MacHomebrew {
            program: Some(PathBuf::from("/bin/sleep")),
            timeout: Duration::from_secs(2),
        };
        let cancel = CancellationToken::new();
        let cancel_later = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            cancel_later.cancel();
        });
        assert_eq!(
            cancel_adapter.run(&["30".into()], &cancel).await,
            Err(PackageError::Cancelled)
        );
    }

    #[tokio::test]
    async fn read_queries_disable_homebrew_auto_update() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().unwrap();
        let program = temp.path().join("brew");
        std::fs::write(
            &program,
            "#!/bin/sh\nprintf '%s\\n' \"${HOMEBREW_NO_AUTO_UPDATE:-missing}\"\n",
        )
        .unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();

        let adapter = MacHomebrew::with_program(program);
        assert_eq!(
            adapter.run(&[], &CancellationToken::new()).await.unwrap(),
            b"1\n"
        );
    }

    #[tokio::test]
    async fn search_keeps_formula_matches_when_the_cask_side_is_empty() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().unwrap();
        let program = temp.path().join("brew");
        std::fs::write(
            &program,
            r##"#!/bin/sh
case "$2" in
  --formula) printf '%s\n' 'luma-e2e-probe' ;;
  --cask)
    printf '%s\n' 'Error: No formulae or casks found for "luma-e2e-probe".' >&2
    exit 1
    ;;
esac
"##,
        )
        .unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();

        let records = MacHomebrew::with_program(program)
            .query(
                PackageQuery::Search("luma-e2e-probe".into()),
                10,
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].name, "luma-e2e-probe");
        assert_eq!(records[0].kind, PackageKind::Formula);
    }

    #[tokio::test]
    async fn search_does_not_hide_unrelated_command_failures() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().unwrap();
        let program = temp.path().join("brew");
        std::fs::write(
            &program,
            "#!/bin/sh\nprintf '%s\\n' 'Error: tap is unavailable' >&2\nexit 1\n",
        )
        .unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();

        assert!(matches!(
            MacHomebrew::with_program(program)
                .query(
                    PackageQuery::Search("luma-e2e-probe".into()),
                    10,
                    CancellationToken::new(),
                )
                .await,
            Err(PackageError::CommandFailed(message))
                if message == "Error: tap is unavailable"
        ));
    }

    #[test]
    fn command_arguments_are_exact() {
        assert_eq!(
            string_args(&["info", "--json=v2", "--installed"]),
            ["info", "--json=v2", "--installed"]
        );
        assert_eq!(
            mutation_args(PackageMutation::Upgrade, "zed", PackageKind::Cask),
            ["upgrade", "--cask", "zed"]
        );
    }
}
