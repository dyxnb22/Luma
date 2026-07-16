use luma_application::ProfileStoreError;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

use super::store::MacProfileStore;
use super::{MAX_PROFILE_BYTES, MAX_REDIRECTS};

fn subscription_curl_args(https: bool) -> Vec<&'static str> {
    let protocol = if https { "=https" } else { "=http" };
    let max_redirects = if https { MAX_REDIRECTS } else { "0" };
    vec![
        // curl only honors this when it is its first option; do not inherit user curlrc behavior
        // that could weaken TLS or write a subscription URL to a trace file.
        "--disable",
        "--silent",
        "--show-error",
        "--fail",
        "--location",
        "--max-redirs",
        max_redirects,
        "--proto",
        protocol,
        "--proto-redir",
        protocol,
        "--connect-timeout",
        "5",
        "--max-time",
        "20",
        // Keep curl's early Content-Length rejection as a second line of defense. The streaming
        // reader below remains authoritative for chunked or lengthless responses.
        "--max-filesize",
        "524288",
        "--config",
        "-",
    ]
}

async fn read_bounded_output<R: AsyncRead + Unpin>(
    reader: &mut R,
    limit: usize,
) -> Result<Vec<u8>, ProfileStoreError> {
    let mut output = Vec::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .await
            .map_err(|_| ProfileStoreError::Unavailable("subscription request failed".into()))?;
        if read == 0 {
            return Ok(output);
        }
        if output.len().saturating_add(read) > limit {
            return Err(ProfileStoreError::SecurityDenied(
                "profile response exceeds the size limit".into(),
            ));
        }
        output.extend_from_slice(&buffer[..read]);
    }
}
fn curl_config_escape(url: &str) -> Result<String, ProfileStoreError> {
    if url.chars().any(|c| c == '"' || c == '\\' || c.is_control()) {
        return Err(ProfileStoreError::InvalidInput {
            field: "subscription".into(),
            message: "subscription address contains unsupported characters".into(),
        });
    }
    Ok(url.to_string())
}

fn is_loopback_http_url(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("http://") else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let (host, port) = if let Some(rest) = authority.strip_prefix('[') {
        let Some((host, rest)) = rest.split_once(']') else {
            return false;
        };
        (host, rest.strip_prefix(':'))
    } else {
        let mut parts = authority.splitn(2, ':');
        (parts.next().unwrap_or_default(), parts.next())
    };
    if !matches!(host, "localhost" | "127.0.0.1" | "::1") {
        return false;
    }
    port.is_none_or(|port| !port.is_empty() && port.parse::<u16>().is_ok())
}

impl MacProfileStore {
    pub(super) async fn fetch_url(&self, url: &str) -> Result<Vec<u8>, ProfileStoreError> {
        let https = url.starts_with("https://");
        let loopback_http = is_loopback_http_url(url);
        if !https && !loopback_http {
            return Err(ProfileStoreError::SecurityDenied(
                "only HTTPS or loopback HTTP subscriptions are allowed".into(),
            ));
        }
        let mut command = Command::new("curl");
        command
            .args(subscription_curl_args(https))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // Errors are deliberately generic, so retaining arbitrary curl stderr only creates
            // another unbounded buffer with no user-facing value.
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let mut child = command
            .spawn()
            .map_err(|_| ProfileStoreError::Unavailable("subscription request failed".into()))?;
        let escaped_url = curl_config_escape(url)?;
        let Some(mut stdin) = child.stdin.take() else {
            return Err(ProfileStoreError::Unavailable(
                "subscription request failed".into(),
            ));
        };
        stdin
            .write_all(format!("url = \"{}\"\n", escaped_url).as_bytes())
            .await
            .map_err(|_| ProfileStoreError::Unavailable("subscription request failed".into()))?;
        drop(stdin);
        let Some(mut stdout) = child.stdout.take() else {
            return Err(ProfileStoreError::Unavailable(
                "subscription request failed".into(),
            ));
        };

        let completed = tokio::time::timeout(Duration::from_secs(25), async {
            let output = read_bounded_output(&mut stdout, MAX_PROFILE_BYTES as usize).await?;
            let status = child.wait().await.map_err(|_| {
                ProfileStoreError::Unavailable("subscription request failed".into())
            })?;
            if !status.success() {
                return Err(ProfileStoreError::Unavailable(
                    "subscription request failed".into(),
                ));
            }
            Ok(output)
        })
        .await;

        match completed {
            Ok(Ok(output)) => Ok(output),
            Ok(Err(error)) => {
                let _ = child.kill().await;
                Err(error)
            }
            Err(_) => {
                let _ = child.kill().await;
                Err(ProfileStoreError::Timeout)
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::super::MAX_PROFILE_BYTES;
    use super::*;
    use luma_application::ProfileStoreError;
    use tokio::io::AsyncWriteExt;

    #[test]
    fn loopback_http_validation_accepts_ports_but_not_lookalikes() {
        assert!(is_loopback_http_url("http://127.0.0.1:8080/profile"));
        assert!(is_loopback_http_url("http://localhost:8080/profile"));
        assert!(is_loopback_http_url("http://[::1]:8080/profile"));
        assert!(!is_loopback_http_url("http://127.0.0.1.evil/profile"));
        assert!(!is_loopback_http_url("http://192.168.1.2:8080/profile"));
    }

    #[test]
    fn subscription_curl_policy_blocks_protocol_downgrades_and_user_curlrc() {
        let https = subscription_curl_args(true);
        assert_eq!(https.first(), Some(&"--disable"));
        assert!(https
            .windows(2)
            .any(|arguments| arguments == ["--proto", "=https"]));
        assert!(https
            .windows(2)
            .any(|arguments| arguments == ["--proto-redir", "=https"]));
        assert!(https
            .windows(2)
            .any(|arguments| arguments == ["--max-redirs", "3"]));

        let loopback_http = subscription_curl_args(false);
        assert!(loopback_http
            .windows(2)
            .any(|arguments| arguments == ["--proto", "=http"]));
        assert!(loopback_http
            .windows(2)
            .any(|arguments| arguments == ["--proto-redir", "=http"]));
        assert!(loopback_http
            .windows(2)
            .any(|arguments| arguments == ["--max-redirs", "0"]));
    }

    #[tokio::test]
    async fn bounded_subscription_reader_rejects_lengthless_oversized_output() {
        let (mut writer, mut reader) = tokio::io::duplex(1024);
        let writer = tokio::spawn(async move {
            let _ = writer
                .write_all(&vec![b'x'; MAX_PROFILE_BYTES as usize + 1])
                .await;
        });
        assert!(matches!(
            read_bounded_output(&mut reader, MAX_PROFILE_BYTES as usize).await,
            Err(ProfileStoreError::SecurityDenied(_))
        ));
        writer.await.unwrap();
    }
}
