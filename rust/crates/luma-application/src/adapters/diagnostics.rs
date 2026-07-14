use crate::ports::{DiagnosticsError, DiagnosticsSink};
use std::path::{Path, PathBuf};

/// Filesystem diagnostics sink rooted at a logs directory.
pub struct FsDiagnosticsSink {
    root: PathBuf,
}

impl FsDiagnosticsSink {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn luma_next_default() -> Result<Self, DiagnosticsError> {
        let root = luma_storage::luma_next_diagnostics_dir()
            .map_err(|e| DiagnosticsError::Unavailable(e.to_string()))?;
        Ok(Self::new(root))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl DiagnosticsSink for FsDiagnosticsSink {
    fn write_export(&self, file_name: &str, body: &[u8]) -> Result<PathBuf, DiagnosticsError> {
        std::fs::create_dir_all(&self.root).map_err(|e| DiagnosticsError::Io(e.to_string()))?;
        let path = self.root.join(file_name);
        std::fs::write(&path, body).map_err(|e| DiagnosticsError::Io(e.to_string()))?;
        Ok(path)
    }
}
