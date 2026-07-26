//! Privacy-filtered, bounded, read-only zsh history adapter.

use async_trait::async_trait;
use luma_application::{
    ShellHistoryEntry, ShellHistoryError, ShellHistoryPort, ShellHistorySnapshot,
    MAX_SHELL_HISTORY_BYTES, MAX_SHELL_HISTORY_COMMAND_BYTES, MAX_SHELL_HISTORY_ENTRIES,
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::sync::CancellationToken;

pub struct MacShellHistory {
    path: PathBuf,
}

impl MacShellHistory {
    pub fn system_default() -> Result<Self, ShellHistoryError> {
        let path = dirs::home_dir()
            .ok_or(ShellHistoryError::NotConfigured)?
            .join(".zsh_history");
        Ok(Self { path })
    }

    pub fn with_path(path: PathBuf) -> Self {
        Self { path }
    }
}

#[async_trait]
impl ShellHistoryPort for MacShellHistory {
    async fn read(
        &self,
        cancel: CancellationToken,
    ) -> Result<ShellHistorySnapshot, ShellHistoryError> {
        if cancel.is_cancelled() {
            return Err(ShellHistoryError::Cancelled);
        }
        let mut file = match tokio::fs::File::open(&self.path).await {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(ShellHistoryError::NotConfigured)
            }
            Err(error) => return Err(ShellHistoryError::Unavailable(error.to_string())),
        };
        let metadata = file
            .metadata()
            .await
            .map_err(|error| ShellHistoryError::Unavailable(error.to_string()))?;
        if !metadata.is_file() {
            return Err(ShellHistoryError::Unavailable(
                "history path is not a regular file".into(),
            ));
        }
        let offset = metadata
            .len()
            .saturating_sub(MAX_SHELL_HISTORY_BYTES as u64);
        if offset > 0 {
            file.seek(std::io::SeekFrom::Start(offset))
                .await
                .map_err(|error| ShellHistoryError::Unavailable(error.to_string()))?;
        }
        if cancel.is_cancelled() {
            return Err(ShellHistoryError::Cancelled);
        }
        let mut bytes = Vec::new();
        file.take(MAX_SHELL_HISTORY_BYTES as u64)
            .read_to_end(&mut bytes)
            .await
            .map_err(|error| ShellHistoryError::Unavailable(error.to_string()))?;
        if cancel.is_cancelled() {
            return Err(ShellHistoryError::Cancelled);
        }
        if offset > 0 {
            if let Some(first_newline) = bytes.iter().position(|byte| *byte == b'\n') {
                bytes.drain(..=first_newline);
            } else {
                bytes.clear();
            }
        }
        Ok(parse_history(&bytes))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ParsedCommand {
    command: String,
    timestamp: Option<i64>,
    duration_seconds: Option<u64>,
}

fn parse_history(bytes: &[u8]) -> ShellHistorySnapshot {
    let mut lines = bytes.split(|byte| *byte == b'\n').collect::<Vec<_>>();
    if lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    if lines.len() > MAX_SHELL_HISTORY_ENTRIES {
        lines.drain(..lines.len() - MAX_SHELL_HISTORY_ENTRIES);
    }

    let mut parsed = Vec::new();
    let mut extended: Option<ParsedCommand> = None;
    for line in lines {
        let line = String::from_utf8_lossy(line);
        if let Some((timestamp, duration, command)) = parse_extended_header(&line) {
            if let Some(previous) = extended.take() {
                parsed.push(previous);
            }
            extended = Some(ParsedCommand {
                command: command.into(),
                timestamp: Some(timestamp),
                duration_seconds: Some(duration),
            });
        } else if let Some(current) = extended.as_mut() {
            current.command.push('\n');
            current.command.push_str(&line);
        } else if !line.is_empty() {
            parsed.push(ParsedCommand {
                command: line.into_owned(),
                timestamp: None,
                duration_seconds: None,
            });
        }
    }
    if let Some(previous) = extended {
        parsed.push(previous);
    }

    let mut hidden_count = 0;
    let mut entries = Vec::new();
    let mut occurrences = HashMap::<ParsedCommand, usize>::new();
    for parsed in parsed.into_iter().rev() {
        if parsed.command.is_empty()
            || parsed.command.len() > MAX_SHELL_HISTORY_COMMAND_BYTES
            || parsed.command.contains('\0')
            || credential_bearing(&parsed.command)
        {
            hidden_count += 1;
            continue;
        }
        let occurrence = occurrences.entry(parsed.clone()).or_default();
        entries.push(ShellHistoryEntry {
            id: command_id(&parsed, *occurrence),
            command: parsed.command,
            timestamp: parsed.timestamp,
            duration_seconds: parsed.duration_seconds,
        });
        *occurrence += 1;
        if entries.len() >= MAX_SHELL_HISTORY_ENTRIES {
            break;
        }
    }
    ShellHistorySnapshot {
        entries,
        hidden_count,
    }
}

fn parse_extended_header(line: &str) -> Option<(i64, u64, &str)> {
    let rest = line.strip_prefix(": ")?;
    let (timestamp, rest) = rest.split_once(':')?;
    let (duration, command) = rest.split_once(';')?;
    Some((timestamp.parse().ok()?, duration.parse().ok()?, command))
}

fn credential_bearing(command: &str) -> bool {
    if luma_domain::looks_secret(command) {
        return true;
    }
    let lower = command.to_lowercase();
    if [
        "--password",
        "--passwd",
        "--token",
        "--secret",
        "--api-key",
        "--api_key",
        "authorization:",
        "bearer ",
        "private_key",
        "private key",
    ]
    .iter()
    .any(|pattern| lower.contains(pattern))
    {
        return true;
    }
    let tokens = command.split_whitespace().collect::<Vec<_>>();
    for (index, token) in tokens.iter().enumerate() {
        let token = token.trim_matches(|ch| matches!(ch, '\'' | '"' | ';'));
        if matches!(token, "-u" | "--user" | "--proxy-user")
            && tokens
                .get(index + 1)
                .is_some_and(|value| value.trim_matches(['\'', '"']).contains(':'))
        {
            return true;
        }
        if ["--user=", "--proxy-user="].iter().any(|flag| {
            token
                .strip_prefix(flag)
                .is_some_and(|value| value.contains(':'))
        }) {
            return true;
        }
        if ["-u", "-U"].iter().any(|flag| {
            token
                .strip_prefix(flag)
                .is_some_and(|value| !value.is_empty() && value.contains(':'))
        }) {
            return true;
        }
        if let Some((key, _value)) = token.split_once('=') {
            let key = key.to_ascii_uppercase();
            if [
                "TOKEN",
                "SECRET",
                "PASSWORD",
                "PASSWD",
                "API_KEY",
                "PRIVATE_KEY",
            ]
            .iter()
            .any(|sensitive| key.contains(sensitive))
            {
                return true;
            }
        }
    }
    let executable = tokens
        .iter()
        .find(|token| !token.contains('='))
        .and_then(|token| token.rsplit('/').next())
        .unwrap_or_default();
    if matches!(
        executable,
        "mysql" | "mysqldump" | "mariadb" | "mariadb-dump"
    ) && tokens.iter().any(|token| {
        token
            .strip_prefix("-p")
            .is_some_and(|password| !password.is_empty())
    }) {
        return true;
    }
    contains_url_userinfo_password(command)
}

fn contains_url_userinfo_password(command: &str) -> bool {
    let mut remaining = command;
    while let Some(scheme) = remaining.find("://") {
        let after_scheme = &remaining[scheme + 3..];
        let end = after_scheme
            .find(char::is_whitespace)
            .unwrap_or(after_scheme.len());
        let authority = &after_scheme[..end];
        if let Some((userinfo, _host)) = authority.rsplit_once('@') {
            if userinfo
                .split_once(':')
                .is_some_and(|(user, password)| !user.is_empty() && !password.is_empty())
            {
                return true;
            }
        }
        remaining = &after_scheme[end..];
    }
    false
}

fn command_id(command: &ParsedCommand, occurrence: usize) -> String {
    let mut digest = Sha256::new();
    digest.update(command.command.as_bytes());
    digest.update([0]);
    digest.update(command.timestamp.unwrap_or_default().to_le_bytes());
    digest.update(occurrence.to_le_bytes());
    hex::encode(digest.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn parses_plain_extended_multiline_unicode_and_invalid_utf8() {
        let bytes = b"plain command\n: 1700000000:2;echo first\ncontinued\n: 1700000001:0;echo \xe4\xb8\xad\xe6\x96\x87\nbad \xff\n";
        let snapshot = parse_history(bytes);
        assert_eq!(snapshot.entries.len(), 3);
        assert_eq!(snapshot.entries[0].command, "echo 中文\nbad �");
        assert_eq!(snapshot.entries[0].timestamp, Some(1_700_000_001));
        assert_eq!(snapshot.entries[1].command, "echo first\ncontinued");
        assert_eq!(snapshot.entries[2].command, "plain command");
    }

    #[test]
    fn secrets_nul_and_oversized_commands_never_leave_adapter() {
        let oversized = "x".repeat(MAX_SHELL_HISTORY_COMMAND_BYTES + 1);
        let source = format!(
            "echo safe\nexport API_TOKEN=fixture\ncurl -H 'Authorization: Bearer fixture' x\ncurl https://u:p@example.test\ncurl -u user:pass example.test\nmysql -pfixture app\nhas\\0nul\n{oversized}\n"
        )
        .replace("\\0", "\0");
        let snapshot = parse_history(source.as_bytes());
        assert_eq!(snapshot.entries.len(), 1);
        assert_eq!(snapshot.entries[0].command, "echo safe");
        assert_eq!(snapshot.hidden_count, 7);
        let debug = format!("{snapshot:?}");
        for secret in ["API_TOKEN", "Authorization", "u:p@", "has\0nul"] {
            assert!(!debug.contains(secret));
        }
    }

    #[test]
    fn entry_and_byte_caps_are_enforced() {
        let source = (0..2_100)
            .map(|index| format!("echo {index}"))
            .collect::<Vec<_>>()
            .join("\n");
        let snapshot = parse_history(source.as_bytes());
        assert_eq!(snapshot.entries.len(), MAX_SHELL_HISTORY_ENTRIES);
        assert_eq!(snapshot.entries[0].command, "echo 2099");
        assert_eq!(snapshot.entries.last().unwrap().command, "echo 100");
    }

    #[test]
    fn unique_entry_ids_survive_unrelated_history_insertions() {
        let original = parse_history(b"echo one\necho two\n");
        let changed = parse_history(b"echo unrelated\necho one\necho two\n");
        let original_one = original
            .entries
            .iter()
            .find(|entry| entry.command == "echo one")
            .unwrap();
        let changed_one = changed
            .entries
            .iter()
            .find(|entry| entry.command == "echo one")
            .unwrap();
        assert_eq!(original_one.id, changed_one.id);
    }

    #[tokio::test]
    async fn reads_only_tail_and_missing_is_not_configured() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("history");
        let mut source = vec![b'x'; MAX_SHELL_HISTORY_BYTES + 100];
        source.extend_from_slice(b"\necho recent\n");
        fs::write(&path, source).unwrap();
        let snapshot = MacShellHistory::with_path(path)
            .read(CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(snapshot.entries[0].command, "echo recent");

        assert_eq!(
            MacShellHistory::with_path(temp.path().join("missing"))
                .read(CancellationToken::new())
                .await,
            Err(ShellHistoryError::NotConfigured)
        );
    }

    #[tokio::test]
    async fn pre_cancel_avoids_file_read() {
        let cancel = CancellationToken::new();
        cancel.cancel();
        assert_eq!(
            MacShellHistory::with_path(PathBuf::from("/not/read"))
                .read(cancel)
                .await,
            Err(ShellHistoryError::Cancelled)
        );
    }
}
