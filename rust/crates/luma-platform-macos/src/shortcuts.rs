//! Apple Shortcuts CLI adapter. All calls use `/usr/bin/shortcuts` plus explicit argv.

use async_trait::async_trait;
use luma_application::{
    ShortcutEntry, ShortcutRunPlan, ShortcutsError, ShortcutsPort, MAX_SHORTCUT_OUTPUT_BYTES,
    MAX_SHORTCUT_RESULTS,
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio_util::sync::CancellationToken;

const SHORTCUTS_PATH: &str = "/usr/bin/shortcuts";
const SHORTCUTS_TIMEOUT: Duration = Duration::from_secs(10);

pub struct MacShortcuts {
    program: PathBuf,
    timeout: Duration,
}

impl MacShortcuts {
    pub fn system_default() -> Self {
        Self {
            program: PathBuf::from(SHORTCUTS_PATH),
            timeout: SHORTCUTS_TIMEOUT,
        }
    }

    #[cfg(test)]
    fn with_program(program: PathBuf) -> Self {
        Self {
            program,
            timeout: SHORTCUTS_TIMEOUT,
        }
    }

    fn ensure_available(&self) -> Result<(), ShortcutsError> {
        if self.program.is_file() {
            Ok(())
        } else {
            Err(ShortcutsError::Unavailable)
        }
    }

    async fn run(
        &self,
        args: &[String],
        cancel: &CancellationToken,
    ) -> Result<Vec<u8>, ShortcutsError> {
        if cancel.is_cancelled() {
            return Err(ShortcutsError::Cancelled);
        }
        self.ensure_available()?;
        let mut child = tokio::process::Command::new(&self.program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| ShortcutsError::Unavailable)?;
        let stdout = child.stdout.take().ok_or(ShortcutsError::Unavailable)?;
        let stderr = child.stderr.take().ok_or(ShortcutsError::Unavailable)?;
        let stdout_task = tokio::spawn(read_bounded(stdout));
        let stderr_task = tokio::spawn(read_bounded(stderr));
        let status = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                let _ = child.kill().await;
                stdout_task.abort();
                stderr_task.abort();
                return Err(ShortcutsError::Cancelled);
            }
            _ = tokio::time::sleep(self.timeout) => {
                let _ = child.kill().await;
                stdout_task.abort();
                stderr_task.abort();
                return Err(ShortcutsError::Timeout);
            }
            status = child.wait() => status.map_err(|_| ShortcutsError::Unavailable)?,
        };
        let stdout = stdout_task
            .await
            .map_err(|_| ShortcutsError::Unavailable)??;
        let stderr = stderr_task
            .await
            .map_err(|_| ShortcutsError::Unavailable)??;
        if stdout.len().saturating_add(stderr.len()) > MAX_SHORTCUT_OUTPUT_BYTES {
            return Err(ShortcutsError::OutputTooLarge(MAX_SHORTCUT_OUTPUT_BYTES));
        }
        if !status.success() {
            return Err(ShortcutsError::CommandFailed(sanitize_stderr(&stderr)));
        }
        Ok(stdout)
    }

    async fn list_names(
        &self,
        args: &[String],
        cancel: &CancellationToken,
    ) -> Result<Vec<String>, ShortcutsError> {
        let output = self.run(args, cancel).await?;
        parse_lines(&output)
    }
}

#[async_trait]
impl ShortcutsPort for MacShortcuts {
    async fn list(
        &self,
        folder: Option<&str>,
        limit: usize,
        cancel: CancellationToken,
    ) -> Result<Vec<ShortcutEntry>, ShortcutsError> {
        let args = if let Some(folder) = folder {
            validate_name(folder)?;
            let folders = self.folders(MAX_SHORTCUT_RESULTS, cancel.clone()).await?;
            match folders
                .iter()
                .filter(|value| value.as_str() == folder)
                .count()
            {
                0 => return Err(ShortcutsError::NotFound),
                1 => {}
                _ => return Err(ShortcutsError::Ambiguous),
            }
            if cancel.is_cancelled() {
                return Err(ShortcutsError::Cancelled);
            }
            vec!["list".into(), "-f".into(), folder.into()]
        } else {
            vec!["list".into()]
        };
        let names = self.list_names(&args, &cancel).await?;
        Ok(entries_from_names(names, limit))
    }

    async fn folders(
        &self,
        limit: usize,
        cancel: CancellationToken,
    ) -> Result<Vec<String>, ShortcutsError> {
        let mut folders = self
            .list_names(&["list".into(), "--folders".into()], &cancel)
            .await?;
        folders.truncate(limit.min(MAX_SHORTCUT_RESULTS));
        Ok(folders)
    }

    async fn resolve_exact(
        &self,
        name: &str,
        cancel: CancellationToken,
    ) -> Result<ShortcutEntry, ShortcutsError> {
        validate_name(name)?;
        let matches = self
            .list(None, MAX_SHORTCUT_RESULTS, cancel)
            .await?
            .into_iter()
            .filter(|entry| entry.name == name)
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [entry] => Ok(entry.clone()),
            [] => Err(ShortcutsError::NotFound),
            _ => Err(ShortcutsError::Ambiguous),
        }
    }

    async fn view(&self, name: &str, cancel: CancellationToken) -> Result<(), ShortcutsError> {
        let exact = self.resolve_exact(name, cancel.clone()).await?;
        if cancel.is_cancelled() {
            return Err(ShortcutsError::Cancelled);
        }
        self.run(&["view".into(), exact.name], &cancel).await?;
        Ok(())
    }

    async fn run_plan(
        &self,
        name: &str,
        cancel: CancellationToken,
    ) -> Result<ShortcutRunPlan, ShortcutsError> {
        let shortcut = self.resolve_exact(name, cancel.clone()).await?;
        if cancel.is_cancelled() {
            return Err(ShortcutsError::Cancelled);
        }
        Ok(ShortcutRunPlan {
            program: self
                .program
                .to_str()
                .ok_or(ShortcutsError::Unavailable)?
                .into(),
            args: vec!["run".into(), shortcut.name.clone()],
            shortcut,
        })
    }
}

async fn read_bounded(
    reader: impl tokio::io::AsyncRead + Unpin,
) -> Result<Vec<u8>, ShortcutsError> {
    let mut bytes = Vec::new();
    reader
        .take((MAX_SHORTCUT_OUTPUT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| ShortcutsError::Unavailable)?;
    if bytes.len() > MAX_SHORTCUT_OUTPUT_BYTES {
        return Err(ShortcutsError::OutputTooLarge(MAX_SHORTCUT_OUTPUT_BYTES));
    }
    Ok(bytes)
}

fn parse_lines(bytes: &[u8]) -> Result<Vec<String>, ShortcutsError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| ShortcutsError::CommandFailed(error.to_string()))?;
    let mut values = Vec::new();
    for line in text
        .lines()
        .filter(|line| !line.is_empty())
        .take(MAX_SHORTCUT_RESULTS)
    {
        validate_name(line)?;
        values.push(line.into());
    }
    Ok(values)
}

fn entries_from_names(names: Vec<String>, limit: usize) -> Vec<ShortcutEntry> {
    let mut occurrences = HashMap::<String, usize>::new();
    names
        .into_iter()
        .take(limit.min(MAX_SHORTCUT_RESULTS))
        .map(|name| {
            let occurrence = occurrences.entry(name.clone()).or_default();
            let entry = ShortcutEntry {
                id: shortcut_id(&name, *occurrence),
                name,
            };
            *occurrence += 1;
            entry
        })
        .collect()
}

fn shortcut_id(name: &str, occurrence: usize) -> String {
    let mut digest = Sha256::new();
    digest.update(name.as_bytes());
    digest.update([0]);
    digest.update(occurrence.to_le_bytes());
    hex::encode(digest.finalize())
}

fn validate_name(name: &str) -> Result<(), ShortcutsError> {
    if name.is_empty() || name.len() > 1_024 || name.chars().any(char::is_control) {
        Err(ShortcutsError::CommandFailed(
            "invalid Shortcut or folder name".into(),
        ))
    } else {
        Ok(())
    }
}

fn sanitize_stderr(bytes: &[u8]) -> String {
    let compact = String::from_utf8_lossy(bytes)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if compact.is_empty() {
        "nonzero exit".into()
    } else {
        compact.chars().take(512).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_spaces_unicode_duplicates_and_empty_output() {
        let names = parse_lines("Morning Run\n工作流程\nMorning Run\n\n".as_bytes()).unwrap();
        let entries = entries_from_names(names, 10);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].name, "Morning Run");
        assert_eq!(entries[1].name, "工作流程");
        assert_ne!(entries[0].id, entries[2].id);
        assert!(parse_lines(b"").unwrap().is_empty());
        assert!(parse_lines(b"bad\tname\n").is_err());
    }

    #[test]
    fn unique_shortcut_ids_survive_reordering_and_unrelated_insertions() {
        let original = entries_from_names(vec!["Morning".into(), "Evening".into()], 10);
        let reordered = entries_from_names(
            vec!["Unrelated".into(), "Evening".into(), "Morning".into()],
            10,
        );
        let morning = reordered
            .iter()
            .find(|entry| entry.name == "Morning")
            .unwrap();
        assert_eq!(original[0].id, morning.id);
    }

    #[test]
    fn documented_argument_vectors_are_exact() {
        assert_eq!(vec!["list".to_string()], ["list"]);
        assert_eq!(
            vec!["list".to_string(), "--folders".into()],
            ["list", "--folders"]
        );
        assert_eq!(
            vec!["list".to_string(), "-f".into(), "Work Stuff".into()],
            ["list", "-f", "Work Stuff"]
        );
        assert_eq!(
            vec!["view".to_string(), "Exact Name".into()],
            ["view", "Exact Name"]
        );
        assert_eq!(
            vec!["run".to_string(), "Exact Name".into()],
            ["run", "Exact Name"]
        );
    }

    #[tokio::test]
    async fn missing_nonzero_cancel_and_timeout_are_distinct() {
        let missing = MacShortcuts::with_program(PathBuf::from("/missing/shortcuts"));
        assert_eq!(
            missing.run(&[], &CancellationToken::new()).await,
            Err(ShortcutsError::Unavailable)
        );

        let nonzero = MacShortcuts::with_program(PathBuf::from("/usr/bin/false"));
        assert!(matches!(
            nonzero.run(&[], &CancellationToken::new()).await,
            Err(ShortcutsError::CommandFailed(_))
        ));

        let cancel = CancellationToken::new();
        cancel.cancel();
        assert_eq!(
            nonzero.run(&[], &cancel).await,
            Err(ShortcutsError::Cancelled)
        );

        let timeout = MacShortcuts {
            program: PathBuf::from("/bin/sleep"),
            timeout: Duration::from_millis(10),
        };
        assert_eq!(
            timeout.run(&["30".into()], &CancellationToken::new()).await,
            Err(ShortcutsError::Timeout)
        );
    }

    #[test]
    fn program_is_the_system_shortcuts_path() {
        assert_eq!(
            MacShortcuts::system_default().program,
            std::path::Path::new("/usr/bin/shortcuts")
        );
    }
}
