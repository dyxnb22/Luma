use super::NotesModule;
use luma_domain::SearchItem;
use std::path::Path;

fn read_bounded_utf8_no_follow(path: &Path, max_bytes: usize) -> Option<String> {
    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::io::Read;
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::io::FromRawFd;

        #[cfg(target_os = "macos")]
        const O_RDONLY: i32 = 0;
        #[cfg(target_os = "macos")]
        const O_NOFOLLOW: i32 = 0x0100;
        #[cfg(target_os = "linux")]
        const O_RDONLY: i32 = 0;
        #[cfg(target_os = "linux")]
        const O_NOFOLLOW: i32 = 0o400000;

        extern "C" {
            fn open(path: *const i8, oflag: i32, ...) -> i32;
        }

        let cpath = CString::new(path.as_os_str().as_bytes()).ok()?;
        let fd = unsafe { open(cpath.as_ptr(), O_RDONLY | O_NOFOLLOW) };
        if fd < 0 {
            return None;
        }
        let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
        let mut buf = vec![0u8; max_bytes];
        let n = file.read(&mut buf).ok()?;
        buf.truncate(n);
        Some(String::from_utf8_lossy(&buf).into_owned())
    }
    #[cfg(not(unix))]
    {
        use std::io::Read;
        let mut file = std::fs::File::open(path).ok()?;
        let mut buf = vec![0u8; max_bytes];
        let n = file.read(&mut buf).ok()?;
        buf.truncate(n);
        Some(String::from_utf8_lossy(&buf).into_owned())
    }
}

fn subtitle_path(subtitle: &str) -> String {
    subtitle.split(" — ").next().unwrap_or(subtitle).to_string()
}

fn format_notes_directory_preview(dir: &Path) -> String {
    const MAX: usize = 40;
    let Ok(rd) = std::fs::read_dir(dir) else {
        return format!("Cannot read folder:\n  {}", dir.display());
    };
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    for entry in rd.flatten() {
        let path = entry.path();
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.file_type().is_symlink() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();
        if name.starts_with('.') {
            continue;
        }
        if meta.file_type().is_dir() {
            dirs.push(name);
        } else if meta.file_type().is_file() && name.to_ascii_lowercase().ends_with(".md") {
            files.push(name);
        }
    }
    dirs.sort_by_cached_key(|s| s.to_ascii_lowercase());
    files.sort_by_cached_key(|s| s.to_ascii_lowercase());
    format_directory_children(&dirs, &files, MAX)
}

fn format_directory_children(dirs: &[String], files: &[String], max: usize) -> String {
    let total = dirs.len() + files.len();
    if total == 0 {
        return "Empty folder".into();
    }
    let mut out = format!("{total} item(s):\n");
    let mut shown = 0usize;
    for d in dirs {
        if shown >= max {
            break;
        }
        out.push_str(&format!("  {d}/\n"));
        shown += 1;
    }
    for f in files {
        if shown >= max {
            break;
        }
        out.push_str(&format!("  {f}\n"));
        shown += 1;
    }
    if shown < total {
        out.push_str(&format!("  … +{} more\n", total - shown));
    }
    out.trim_end().to_string()
}

impl NotesModule {
    pub(super) async fn preview(&self, result: &SearchItem) -> Option<String> {
        let root = self.root.read().await.clone()?;
        if result.kind == "directory" || result.id.as_str().starts_with("browse:n:") {
            let path_part = subtitle_path(result.subtitle.as_deref()?);
            let path = Self::resolve_under_root_for_browse(&root, Path::new(&path_part)).ok()?;
            return Some(format_notes_directory_preview(&path));
        }
        let path_part = subtitle_path(result.subtitle.as_deref()?);
        let path = Self::resolve_under_root(&root, Path::new(&path_part)).ok()?;
        let rel = path
            .strip_prefix(&root)
            .ok()
            .map(|p| p.to_string_lossy().replace('\\', "/"));

        let mut out = String::new();
        if let Some(rel) = &rel {
            if let Ok(Some(doc)) = self.index.get_document(rel) {
                out.push_str(&format!("# {}\n", doc.title));
                out.push_str(&format!("path: {}\n", doc.relative_path));
                out.push_str(&format!("mtime: {}\n", doc.mtime_unix));
                out.push_str(&format!("size: {}\n", doc.size_bytes));
                if !doc.tags.is_empty() {
                    out.push_str(&format!("tags: {}\n", doc.tags.join(", ")));
                }
                if !doc.outbound.is_empty() {
                    out.push_str("outbound:\n");
                    for l in doc.outbound.iter().take(12) {
                        out.push_str(&format!("  - {} ({})\n", l.raw_href, l.kind));
                    }
                }
                if !doc.backlinks.is_empty() {
                    out.push_str("backlinks:\n");
                    for l in doc.backlinks.iter().take(12) {
                        out.push_str(&format!("  - {}\n", l.target_path));
                    }
                }
                out.push('\n');
            }
        }

        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            return if out.is_empty() {
                result.subtitle.clone()
            } else {
                Some(out)
            };
        };
        if meta.file_type().is_symlink() {
            return if out.is_empty() { None } else { Some(out) };
        }
        if meta.file_type().is_dir() {
            return Some(format_notes_directory_preview(&path));
        }
        if !meta.file_type().is_file() {
            return if out.is_empty() {
                result.subtitle.clone()
            } else {
                Some(out)
            };
        }
        let path_for_read = path.clone();
        if let Ok(Some(body)) =
            tokio::task::spawn_blocking(move || read_bounded_utf8_no_follow(&path_for_read, 4_096))
                .await
        {
            out.push_str(&body);
        }
        Some(out)
    }
}
