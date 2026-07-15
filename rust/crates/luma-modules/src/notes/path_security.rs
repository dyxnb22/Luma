use super::NotesModule;
use std::path::{Component, Path, PathBuf};

impl NotesModule {
    /// True when `candidate` exists and its canonical path is under canonical `root`.
    pub(crate) fn contained(root: &Path, candidate: &Path) -> bool {
        let Ok(root) = root.canonicalize() else {
            return false;
        };
        let Ok(cand) = candidate.canonicalize() else {
            return false;
        };
        cand.starts_with(&root)
    }

    /// Resolve a create/open target under `root` without allowing escape via `..`,
    /// absolute paths outside the root, or symlink redirection. Call **before** any write.
    /// Rejects the notes root itself (create/write must not overwrite the workspace root).
    pub(crate) fn resolve_under_root(root: &Path, candidate: &Path) -> Result<PathBuf, String> {
        let resolved = Self::resolve_under_root_inner(root, candidate)?;
        let root_canon = root
            .canonicalize()
            .map_err(|e| format!("notes root not accessible: {e}"))?;
        if resolved == root_canon {
            return Err("refusing to overwrite notes root itself".into());
        }
        Ok(resolved)
    }

    /// Like [`resolve_under_root`], but allows `candidate` to resolve to the notes root
    /// itself (needed for `n browse` / `n browse <root>`).
    pub(crate) fn resolve_under_root_for_browse(
        root: &Path,
        candidate: &Path,
    ) -> Result<PathBuf, String> {
        Self::resolve_under_root_inner(root, candidate)
    }

    fn resolve_under_root_inner(root: &Path, candidate: &Path) -> Result<PathBuf, String> {
        let root_canon = root
            .canonicalize()
            .map_err(|e| format!("notes root not accessible: {e}"))?;

        let absolute = if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            for c in candidate.components() {
                match c {
                    Component::ParentDir => {
                        return Err("path escapes notes root (..)".into());
                    }
                    Component::RootDir | Component::Prefix(_) => {
                        return Err("absolute path segments not allowed in relative note id".into());
                    }
                    Component::CurDir | Component::Normal(_) => {}
                }
            }
            root_canon.join(candidate)
        };

        // Resolve through the longest existing ancestor so `/var` vs `/private/var`
        // and symlink roots compare correctly on macOS.
        let mut existing = absolute.clone();
        let mut missing: Vec<std::ffi::OsString> = Vec::new();
        while !existing.as_os_str().is_empty() && !existing.exists() {
            match existing.file_name() {
                Some(name) => {
                    missing.push(name.to_os_string());
                    existing.pop();
                }
                None => break,
            }
        }
        if missing.iter().any(|p| p == "..") {
            return Err("path escapes notes root (..)".into());
        }
        if !existing.exists() {
            return Err("path has no existing ancestor under notes root".into());
        }
        let mut resolved = existing
            .canonicalize()
            .map_err(|e| format!("cannot resolve path: {e}"))?;
        if !resolved.starts_with(&root_canon) {
            return Err("path escapes notes root".into());
        }
        for part in missing.into_iter().rev() {
            if part == ".." {
                return Err("path escapes notes root (..)".into());
            }
            if part == "." {
                continue;
            }
            resolved.push(part);
            if resolved.exists() {
                let canon = resolved
                    .canonicalize()
                    .map_err(|e| format!("cannot resolve path: {e}"))?;
                if !canon.starts_with(&root_canon) {
                    return Err("symlink escapes notes root".into());
                }
                resolved = canon;
            } else if !resolved.starts_with(&root_canon) {
                // Lexical push stayed under root_canon as Path prefix (both absolute).
                return Err("path escapes notes root".into());
            }
        }
        if !resolved.starts_with(&root_canon) {
            return Err("path escapes notes root".into());
        }
        Ok(resolved)
    }

    /// Create parents and the new file via descriptor-relative `openat` / `mkdirat`
    /// with `O_NOFOLLOW` on every step so a swapped parent symlink cannot be followed.
    pub(crate) fn create_new_contained(
        root: &Path,
        candidate: &Path,
    ) -> Result<(PathBuf, std::fs::File), String> {
        let path = Self::resolve_under_root(root, candidate)?;
        let root_canon = root
            .canonicalize()
            .map_err(|e| format!("root canonicalize: {e}"))?;
        let rel = path
            .strip_prefix(&root_canon)
            .map_err(|_| "path not under notes root".to_string())?;
        let comps: Vec<_> = rel
            .components()
            .filter_map(|c| match c {
                Component::Normal(s) => Some(s.to_os_string()),
                _ => None,
            })
            .collect();
        if comps.is_empty() {
            return Err("refusing to overwrite notes root itself".into());
        }

        #[cfg(unix)]
        {
            use std::ffi::CString;
            use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
            use std::os::unix::ffi::OsStrExt;

            #[cfg(target_os = "macos")]
            const O_DIRECTORY: i32 = 0x0010_0000;
            #[cfg(target_os = "macos")]
            const O_NOFOLLOW: i32 = 0x0100;
            #[cfg(target_os = "macos")]
            const O_CREAT: i32 = 0x0200;
            #[cfg(target_os = "macos")]
            const O_EXCL: i32 = 0x0800;
            #[cfg(all(unix, not(target_os = "macos")))]
            const O_DIRECTORY: i32 = 0o200000;
            #[cfg(all(unix, not(target_os = "macos")))]
            const O_NOFOLLOW: i32 = 0o400000;
            #[cfg(all(unix, not(target_os = "macos")))]
            const O_CREAT: i32 = 0o100;
            #[cfg(all(unix, not(target_os = "macos")))]
            const O_EXCL: i32 = 0o200;
            const O_RDONLY: i32 = 0;
            const O_WRONLY: i32 = 1;

            extern "C" {
                fn open(path: *const i8, oflag: i32, ...) -> i32;
                fn openat(dirfd: i32, path: *const i8, oflag: i32, ...) -> i32;
                fn mkdirat(dirfd: i32, path: *const i8, mode: u32) -> i32;
            }

            let root_c = CString::new(root_canon.as_os_str().as_bytes())
                .map_err(|_| "root path contains NUL".to_string())?;
            let root_fd = unsafe { open(root_c.as_ptr(), O_RDONLY | O_DIRECTORY | O_NOFOLLOW) };
            if root_fd < 0 {
                return Err(format!("open root: {}", std::io::Error::last_os_error()));
            }
            let mut dir_fd = unsafe { OwnedFd::from_raw_fd(root_fd) };

            for name in &comps[..comps.len() - 1] {
                let cname = CString::new(name.as_bytes())
                    .map_err(|_| "path component contains NUL".to_string())?;
                let mut child = unsafe {
                    openat(
                        dir_fd.as_raw_fd(),
                        cname.as_ptr(),
                        O_RDONLY | O_DIRECTORY | O_NOFOLLOW,
                    )
                };
                if child < 0 {
                    let err = std::io::Error::last_os_error();
                    if err.kind() == std::io::ErrorKind::NotFound {
                        let mk = unsafe { mkdirat(dir_fd.as_raw_fd(), cname.as_ptr(), 0o755) };
                        if mk != 0 {
                            return Err(format!("mkdirat: {}", std::io::Error::last_os_error()));
                        }
                        child = unsafe {
                            openat(
                                dir_fd.as_raw_fd(),
                                cname.as_ptr(),
                                O_RDONLY | O_DIRECTORY | O_NOFOLLOW,
                            )
                        };
                        if child < 0 {
                            return Err(format!(
                                "openat after mkdir: {}",
                                std::io::Error::last_os_error()
                            ));
                        }
                    } else {
                        return Err(format!("openat parent: {err}"));
                    }
                }
                dir_fd = unsafe { OwnedFd::from_raw_fd(child) };
            }

            let file_name = &comps[comps.len() - 1];
            let cfile = CString::new(file_name.as_bytes())
                .map_err(|_| "filename contains NUL".to_string())?;
            let file_fd = unsafe {
                openat(
                    dir_fd.as_raw_fd(),
                    cfile.as_ptr(),
                    O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW,
                    0o644,
                )
            };
            if file_fd < 0 {
                return Err(format!(
                    "openat create: {}",
                    std::io::Error::last_os_error()
                ));
            }
            let file = unsafe { std::fs::File::from_raw_fd(file_fd) };
            Ok((path, file))
        }

        #[cfg(not(unix))]
        {
            let _ = comps;
            Err("contained create requires Unix openat".into())
        }
    }
}
