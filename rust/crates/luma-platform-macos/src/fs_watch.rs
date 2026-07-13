//! Directory change detection.
//! Prefer `notify` (FSEvents on macOS). Polling remains as a fallback helper.

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::SystemTime;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug, Default)]
pub struct DirFingerprint {
    pub file_count: u64,
    pub latest_mtime_secs: u64,
}

impl DirFingerprint {
    pub fn scan(root: &Path) -> Self {
        let mut file_count = 0u64;
        let mut latest = 0u64;
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in rd.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().and_then(|e| e.to_str()) != Some("md") {
                    continue;
                }
                file_count += 1;
                if let Ok(meta) = path.metadata() {
                    if let Ok(modified) = meta.modified() {
                        if let Ok(dur) = modified.duration_since(SystemTime::UNIX_EPOCH) {
                            latest = latest.max(dur.as_secs());
                        }
                    }
                }
            }
        }
        Self {
            file_count,
            latest_mtime_secs: latest,
        }
    }
}

/// Poll `root` until cancel; invoke `on_change` when fingerprint changes.
pub async fn poll_markdown_root<F, Fut>(
    root: PathBuf,
    interval_ms: u64,
    cancel: CancellationToken,
    mut on_change: F,
) where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let mut last = DirFingerprint::scan(&root);
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = tokio::time::sleep(std::time::Duration::from_millis(interval_ms)) => {
                let next = DirFingerprint::scan(&root);
                if next.file_count != last.file_count
                    || next.latest_mtime_secs != last.latest_mtime_secs
                {
                    last = next;
                    on_change().await;
                }
            }
        }
    }
}

/// Watch `root` via notify (FSEvents on macOS). Debounces bursts; cancel stops the watcher.
pub async fn watch_markdown_root<F, Fut>(root: PathBuf, cancel: CancellationToken, mut on_change: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let (tx, rx) = mpsc::channel();
    let mut watcher = match RecommendedWatcher::new(
        move |res: Result<notify::Event, notify::Error>| {
            let _ = tx.send(res);
        },
        notify::Config::default(),
    ) {
        Ok(w) => w,
        Err(_) => {
            // Fall back to polling if watcher cannot start.
            return poll_markdown_root(root, 1500, cancel, on_change).await;
        }
    };

    if watcher.watch(&root, RecursiveMode::Recursive).is_err() {
        return poll_markdown_root(root, 1500, cancel, on_change).await;
    }

    let mut dirty = false;
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = tokio::time::sleep(std::time::Duration::from_millis(250)) => {
                // Drain notify channel without blocking the async runtime long.
                let mut saw = false;
                while let Ok(msg) = rx.try_recv() {
                    match msg {
                        Ok(ev) => {
                            if matches!(
                                ev.kind,
                                EventKind::Create(_)
                                    | EventKind::Modify(_)
                                    | EventKind::Remove(_)
                                    | EventKind::Any
                            ) {
                                saw = true;
                            }
                        }
                        Err(_) => {
                            // Overflow / watcher error → force rebuild.
                            saw = true;
                        }
                    }
                }
                if saw {
                    dirty = true;
                } else if dirty {
                    dirty = false;
                    on_change().await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn fingerprint_sees_new_file() {
        let dir = tempdir().unwrap();
        let a = DirFingerprint::scan(dir.path());
        fs::write(dir.path().join("a.md"), "x").unwrap();
        let b = DirFingerprint::scan(dir.path());
        assert_eq!(a.file_count, 0);
        assert_eq!(b.file_count, 1);
    }
}
