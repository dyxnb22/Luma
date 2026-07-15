//! Safe, redacted Profile management port.
//!
//! Raw subscription contents, local YAML, credentials, and controller configuration stay in
//! platform adapters. Only metadata and opaque identifiers cross into the application layer.

use async_trait::async_trait;
use std::path::Path;
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileSource {
    LumaLocal,
    Subscription,
    ClashVerge,
}

impl ProfileSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::LumaLocal => "Luma local",
            Self::Subscription => "HTTPS subscription",
            Self::ClashVerge => "Clash Verge",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileSummary {
    pub id: String,
    pub name: String,
    pub node_count: usize,
    pub group_count: usize,
    pub rule_count: usize,
    pub metadata_available: bool,
    pub updated_at: Option<u64>,
    pub source: ProfileSource,
    pub owned_by_luma: bool,
    pub current: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileImportResult {
    pub summary: ProfileSummary,
    pub source_written: bool,
    pub metadata_updated: bool,
    pub runtime_applied: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProfileStoreError {
    #[error("profile input is invalid ({field}): {message}")]
    InvalidInput { field: String, message: String },
    #[error("profile not found: {0}")]
    NotFound(String),
    #[error("profile operation timed out")]
    Timeout,
    #[error("profile operation is unavailable: {0}")]
    Unavailable(String),
    #[error("profile operation is not configured: {0}")]
    NotConfigured(String),
    #[error("profile operation is unsupported: {0}")]
    Unsupported(String),
    #[error("profile operation was denied for security: {0}")]
    SecurityDenied(String),
    #[error("profile operation conflicted: {0}")]
    Conflict(String),
}

#[async_trait]
pub trait ProfileStorePort: Send + Sync {
    async fn list_profiles(&self) -> Result<Vec<ProfileSummary>, ProfileStoreError>;
    async fn import_subscription(
        &self,
        url: &str,
        suggested_name: Option<&str>,
    ) -> Result<ProfileImportResult, ProfileStoreError>;
    async fn import_local_file(
        &self,
        path: &Path,
        suggested_name: Option<&str>,
    ) -> Result<ProfileImportResult, ProfileStoreError>;
    async fn use_profile(&self, id: &str) -> Result<ProfileImportResult, ProfileStoreError>;
    async fn refresh_profile(&self, id: &str) -> Result<ProfileImportResult, ProfileStoreError>;
    async fn delete_profile(&self, id: &str) -> Result<(), ProfileStoreError>;
}
