use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio_util::sync::CancellationToken;

pub const MAX_PACKAGE_RESULTS: usize = 500;
pub const MAX_PACKAGE_OUTPUT_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackageKind {
    Formula,
    Cask,
}

impl PackageKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Formula => "formula",
            Self::Cask => "cask",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageRecord {
    pub name: String,
    pub kind: PackageKind,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub version: Option<String>,
    pub installed: bool,
    pub outdated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackageQuery {
    Installed,
    Outdated,
    Formulae,
    Casks,
    Search(String),
    Info(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackageMutation {
    Install,
    Upgrade,
    Uninstall,
}

impl PackageMutation {
    pub fn program_arg(self) -> &'static str {
        match self {
            Self::Install => "install",
            Self::Upgrade => "upgrade",
            Self::Uninstall => "uninstall",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageMutationPlan {
    pub program: String,
    pub args: Vec<String>,
    pub package: PackageRecord,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PackageError {
    #[error("Homebrew is not installed")]
    NotConfigured,
    #[error("Homebrew unavailable: {0}")]
    Unavailable(String),
    #[error("Homebrew command timed out")]
    Timeout,
    #[error("Homebrew command failed: {0}")]
    CommandFailed(String),
    #[error("Homebrew output is malformed: {0}")]
    Malformed(String),
    #[error("Homebrew output exceeded the {0}-byte limit")]
    OutputTooLarge(usize),
    #[error("package not found")]
    NotFound,
    #[error("package identity is ambiguous")]
    Ambiguous,
    #[error("package state changed: {0}")]
    Conflict(String),
    #[error("package operation cancelled")]
    Cancelled,
}

#[async_trait]
pub trait PackageManagerPort: Send + Sync {
    async fn query(
        &self,
        query: PackageQuery,
        limit: usize,
        cancel: CancellationToken,
    ) -> Result<Vec<PackageRecord>, PackageError>;

    async fn resolve(
        &self,
        name: &str,
        kind: PackageKind,
        cancel: CancellationToken,
    ) -> Result<PackageRecord, PackageError>;

    async fn mutation_plan(
        &self,
        mutation: PackageMutation,
        name: &str,
        kind: PackageKind,
        cancel: CancellationToken,
    ) -> Result<PackageMutationPlan, PackageError>;
}

/// Controllable Homebrew fake. It records only structured operations and never invokes brew.
pub struct FakePackageManager {
    records: Mutex<Vec<PackageRecord>>,
    error: Mutex<Option<PackageError>>,
    pub queries: Arc<Mutex<Vec<PackageQuery>>>,
    pub mutations: Arc<Mutex<Vec<(PackageMutation, String, PackageKind)>>>,
    program: String,
}

impl FakePackageManager {
    pub fn new(records: Vec<PackageRecord>) -> Self {
        Self {
            records: Mutex::new(records),
            error: Mutex::new(None),
            queries: Arc::new(Mutex::new(Vec::new())),
            mutations: Arc::new(Mutex::new(Vec::new())),
            program: "/fixture/brew".into(),
        }
    }

    pub fn fail_with(&self, error: PackageError) {
        *self.error.lock().expect("package error lock") = Some(error);
    }

    pub fn replace(&self, records: Vec<PackageRecord>) {
        *self.records.lock().expect("package records lock") = records;
    }

    fn take_error(&self) -> Result<(), PackageError> {
        match self.error.lock().expect("package error lock").take() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

#[async_trait]
impl PackageManagerPort for FakePackageManager {
    async fn query(
        &self,
        query: PackageQuery,
        limit: usize,
        cancel: CancellationToken,
    ) -> Result<Vec<PackageRecord>, PackageError> {
        if cancel.is_cancelled() {
            return Err(PackageError::Cancelled);
        }
        self.take_error()?;
        self.queries
            .lock()
            .expect("package query lock")
            .push(query.clone());
        let mut records = self.records.lock().expect("package records lock").clone();
        match query {
            PackageQuery::Installed => records.retain(|record| record.installed),
            PackageQuery::Outdated => records.retain(|record| record.outdated),
            PackageQuery::Formulae => records.retain(|record| record.kind == PackageKind::Formula),
            PackageQuery::Casks => records.retain(|record| record.kind == PackageKind::Cask),
            PackageQuery::Search(needle) => {
                let needle = needle.to_lowercase();
                records.retain(|record| {
                    record.name.to_lowercase().contains(&needle)
                        || record
                            .description
                            .as_ref()
                            .is_some_and(|value| value.to_lowercase().contains(&needle))
                });
            }
            PackageQuery::Info(name) => {
                records.retain(|record| record.name.eq_ignore_ascii_case(&name))
            }
        }
        records.sort_by(|a, b| {
            a.name
                .cmp(&b.name)
                .then_with(|| a.kind.label().cmp(b.kind.label()))
        });
        records.truncate(limit.min(MAX_PACKAGE_RESULTS));
        Ok(records)
    }

    async fn resolve(
        &self,
        name: &str,
        kind: PackageKind,
        cancel: CancellationToken,
    ) -> Result<PackageRecord, PackageError> {
        if cancel.is_cancelled() {
            return Err(PackageError::Cancelled);
        }
        self.take_error()?;
        self.records
            .lock()
            .expect("package records lock")
            .iter()
            .find(|record| record.name == name && record.kind == kind)
            .cloned()
            .ok_or(PackageError::NotFound)
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
        self.take_error()?;
        let package = self
            .records
            .lock()
            .expect("package records lock")
            .iter()
            .find(|record| record.name == name && record.kind == kind)
            .cloned()
            .ok_or(PackageError::NotFound)?;
        validate_mutation_state(mutation, &package)?;
        self.mutations
            .lock()
            .expect("package mutation lock")
            .push((mutation, name.into(), kind));
        Ok(PackageMutationPlan {
            program: self.program.clone(),
            args: mutation_args(mutation, name, kind),
            package,
        })
    }
}

pub fn mutation_args(mutation: PackageMutation, name: &str, kind: PackageKind) -> Vec<String> {
    let mut args = vec![mutation.program_arg().into()];
    if mutation == PackageMutation::Uninstall {
        // `brew uninstall` removes only the newest keg when older versions remain. Luma's
        // package-level Uninstall action must leave the formula/cask genuinely uninstalled.
        args.push("--force".into());
    }
    if kind == PackageKind::Cask {
        args.push("--cask".into());
    }
    args.push(name.into());
    args
}

pub fn validate_mutation_state(
    mutation: PackageMutation,
    package: &PackageRecord,
) -> Result<(), PackageError> {
    match mutation {
        PackageMutation::Install if package.installed => Err(PackageError::Conflict(
            "package is already installed".into(),
        )),
        PackageMutation::Upgrade if !package.installed => {
            Err(PackageError::Conflict("package is not installed".into()))
        }
        PackageMutation::Upgrade if !package.outdated => Err(PackageError::Conflict(
            "package is no longer outdated".into(),
        )),
        PackageMutation::Uninstall if !package.installed => Err(PackageError::Conflict(
            "package is no longer installed".into(),
        )),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutation_vectors_are_exact_and_never_shell_interpolated() {
        assert_eq!(
            mutation_args(PackageMutation::Install, "ripgrep", PackageKind::Formula),
            ["install", "ripgrep"]
        );
        assert_eq!(
            mutation_args(
                PackageMutation::Uninstall,
                "hostile; name",
                PackageKind::Cask
            ),
            ["uninstall", "--force", "--cask", "hostile; name"]
        );
    }
}
