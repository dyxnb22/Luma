use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug)]
pub struct TerminalLauncher {
    luma_path: PathBuf,
    launcher_dir: PathBuf,
}

impl TerminalLauncher {
    pub fn new(luma_path: PathBuf, support_dir: PathBuf) -> Self {
        Self {
            luma_path,
            launcher_dir: support_dir.join("launchers"),
        }
    }

    pub fn launch(&self, query: Option<&str>) -> Result<(), String> {
        if !self.luma_path.is_file() {
            return Err(format!(
                "Luma CLI unavailable: {}",
                self.luma_path.display()
            ));
        }
        fs::create_dir_all(&self.launcher_dir).map_err(|e| format!("launcher directory: {e}"))?;
        let name = match query {
            None => "open.command",
            Some("/settings") => "settings.command",
            Some("/wb review due") => "wordbook-review.command",
            Some(_) => return Err("unsupported terminal entry point".into()),
        };
        let path = self.launcher_dir.join(name);
        let body = launcher_body(&self.luma_path, query);
        write_private_executable(&path, body.as_bytes())?;
        Command::new("/usr/bin/open")
            .args(["-a", "Terminal"])
            .arg(&path)
            .status()
            .map_err(|e| format!("open Terminal: {e}"))
            .and_then(|status| {
                if status.success() {
                    Ok(())
                } else {
                    Err(format!("open Terminal exited with {status}"))
                }
            })
    }
}

fn launcher_body(luma_path: &Path, query: Option<&str>) -> String {
    let quoted_luma = shell_quote(&luma_path.display().to_string());
    let args = query
        .map(|query| format!(" --initial-query {}", shell_quote(query)))
        .unwrap_or_default();
    format!("#!/bin/sh\nexec {quoted_luma} tui{args}\n")
}

fn write_private_executable(path: &Path, body: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .map_err(|e| format!("write launcher: {e}"))?;
    file.write_all(body)
        .and_then(|_| file.sync_all())
        .map_err(|e| format!("write launcher: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|e| format!("chmod launcher: {e}"))?;
    }
    Ok(())
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launcher_quotes_path_and_query() {
        let body = launcher_body(Path::new("/tmp/Luma's/luma"), Some("/wb review due"));
        assert!(body.contains("'/tmp/Luma'\\''s/luma'"));
        assert!(body.contains("'/wb review due'"));
    }
}
