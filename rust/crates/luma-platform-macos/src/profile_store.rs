//! Luma-owned Profile storage and the read-only Clash Verge manifest adapter.
//!
//! This adapter is deliberately conservative: it validates YAML before persistence, keeps
//! subscription URLs in Keychain, stores only opaque IDs in JSON, and only ever writes local
//! Profiles marked as Luma-owned.

use async_trait::async_trait;
use luma_application::{
    KeychainError, KeychainPort, ProfileImportResult, ProfileSource, ProfileStoreError,
    ProfileStorePort, ProfileSummary,
};
use luma_storage::luma_next_support_dir;
use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

#[async_trait]
trait RuntimeApplyPort: Send + Sync {
    async fn apply_profile(&self, path: &Path) -> Result<(), ProfileStoreError>;
}

#[async_trait]
impl RuntimeApplyPort for crate::MacMihomoProxyCore {
    async fn apply_profile(&self, path: &Path) -> Result<(), ProfileStoreError> {
        self.apply_profile_file(path)
            .await
            .map_err(|_| ProfileStoreError::Conflict("Mihomo runtime application failed".into()))
    }
}

const MAX_PROFILE_BYTES: u64 = 512 * 1024;
const MAX_REDIRECTS: &str = "3";
const URL_ACCOUNT_PREFIX: &str = "proxy-profile-url:";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StoredProfile {
    id: String,
    name: String,
    node_count: usize,
    group_count: usize,
    rule_count: usize,
    updated_at: Option<u64>,
    source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
struct ProfileIndex {
    profiles: Vec<StoredProfile>,
}

struct ClashSnapshot {
    manifest: PathBuf,
    old_manifest: Vec<u8>,
    target: PathBuf,
    old_target: Option<Vec<u8>>,
}

pub struct MacProfileStore {
    root: PathBuf,
    clash_root: Option<PathBuf>,
    keychain: Arc<dyn KeychainPort>,
    runtime: Option<Arc<dyn RuntimeApplyPort>>,
}

impl MacProfileStore {
    pub fn new(
        keychain: Arc<dyn KeychainPort>,
        runtime: Arc<crate::MacMihomoProxyCore>,
    ) -> Result<Self, ProfileStoreError> {
        let root = luma_next_support_dir()
            .map_err(|e| ProfileStoreError::Unavailable(e.to_string()))?
            .join("proxy-profiles");
        Ok(Self {
            root,
            clash_root: default_clash_root(),
            keychain,
            runtime: Some(runtime),
        })
    }

    #[cfg(test)]
    fn with_paths(
        root: PathBuf,
        clash_root: Option<PathBuf>,
        keychain: Arc<dyn KeychainPort>,
    ) -> Self {
        Self {
            root,
            clash_root,
            keychain,
            runtime: None,
        }
    }

    #[cfg(test)]
    fn with_runtime(mut self, runtime: Arc<dyn RuntimeApplyPort>) -> Self {
        self.runtime = Some(runtime);
        self
    }

    fn index_path(&self) -> PathBuf {
        self.root.join("profiles.json")
    }
    fn source_path(&self, id: &str) -> Result<PathBuf, ProfileStoreError> {
        if !valid_id(id) {
            return Err(ProfileStoreError::SecurityDenied(
                "invalid profile identifier".into(),
            ));
        }
        safe_child(&self.root, &format!("{id}.yaml"))
    }

    fn ensure_root(&self) -> Result<(), ProfileStoreError> {
        fs::create_dir_all(&self.root).map_err(io_error)?;
        set_private_dir_mode(&self.root)
    }

    fn read_index(&self) -> Result<ProfileIndex, ProfileStoreError> {
        let path = self.index_path();
        match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|_| {
                ProfileStoreError::Unavailable("Luma Profile metadata is invalid".into())
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ProfileIndex::default()),
            Err(e) => Err(io_error(e)),
        }
    }

    fn to_summary(stored: StoredProfile, current: bool) -> ProfileSummary {
        ProfileSummary {
            id: stored.id,
            name: stored.name,
            node_count: stored.node_count,
            group_count: stored.group_count,
            rule_count: stored.rule_count,
            metadata_available: true,
            updated_at: stored.updated_at,
            source: if stored.source == "subscription" {
                ProfileSource::Subscription
            } else {
                ProfileSource::LumaLocal
            },
            owned_by_luma: true,
            current,
        }
    }

    fn metadata(
        value: &Value,
        suggested_name: Option<&str>,
        _source: ProfileSource,
    ) -> Result<(String, usize, usize, usize), ProfileStoreError> {
        validate_profile(value)?;
        let name = suggested_name
            .filter(|s| !s.trim().is_empty())
            .or_else(|| value.get("name").and_then(Value::as_str))
            .unwrap_or("Imported Profile");
        let name = safe_name(name)?;
        Ok((
            name,
            sequence_len(value.get("proxies")),
            sequence_len(value.get("proxy-groups")),
            sequence_len(value.get("rules")),
        ))
    }

    async fn import_bytes(
        &self,
        bytes: Vec<u8>,
        suggested_name: Option<&str>,
        source: ProfileSource,
        url: Option<&str>,
        fixed_id: Option<&str>,
    ) -> Result<ProfileImportResult, ProfileStoreError> {
        if bytes.len() as u64 > MAX_PROFILE_BYTES {
            return Err(ProfileStoreError::SecurityDenied(
                "profile response exceeds the size limit".into(),
            ));
        }
        let bytes = normalize_subscription_bytes(bytes)?;
        let raw = String::from_utf8(bytes).map_err(|_| ProfileStoreError::InvalidInput {
            field: "yaml".into(),
            message: "profile is not UTF-8 YAML".into(),
        })?;
        let value: Value =
            serde_yaml::from_str(&raw).map_err(|_| ProfileStoreError::InvalidInput {
                field: "yaml".into(),
                message: "profile is not valid YAML".into(),
            })?;
        let (name, node_count, group_count, rule_count) =
            Self::metadata(&value, suggested_name, source)?;
        self.ensure_root()?;
        let id = fixed_id
            .map(str::to_string)
            .unwrap_or_else(|| new_id(&raw, &name));
        let source_path = self.source_path(&id)?;
        let previous_index = self.read_index()?;
        let old_source = fs::read(&source_path).ok();
        atomic_write(&source_path, raw.as_bytes(), true)?;
        let mut index = previous_index.clone();
        let updated_at = Some(now());
        let stored = StoredProfile {
            id: id.clone(),
            name,
            node_count,
            group_count,
            rule_count,
            updated_at,
            source: if source == ProfileSource::Subscription {
                "subscription".into()
            } else {
                "local".into()
            },
        };
        index.profiles.retain(|p| p.id != id);
        index.profiles.push(stored.clone());
        if let Err(error) = atomic_json(&self.index_path(), &index) {
            let rollback = restore_file(&source_path, old_source.as_deref());
            return Err(rollback_failure(error, rollback));
        }
        if let Some(url) = url {
            if self
                .keychain
                .set_password(&format!("{URL_ACCOUNT_PREFIX}{id}"), url)
                .await
                .is_err()
            {
                let rollback = restore_file(&source_path, old_source.as_deref())
                    .and_then(|_| atomic_json(&self.index_path(), &previous_index));
                return Err(rollback_failure(
                    ProfileStoreError::Unavailable(
                        "subscription address could not be stored in Keychain".into(),
                    ),
                    rollback,
                ));
            }
        }
        if fixed_id.is_some() {
            if let Err(error) = self.sync_registered_clash(&stored, &source_path) {
                let rollback = restore_file(&source_path, old_source.as_deref())
                    .and_then(|_| atomic_json(&self.index_path(), &previous_index));
                return Err(rollback_failure(error, rollback));
            }
        }
        Ok(ProfileImportResult {
            summary: Self::to_summary(stored, false),
            source_written: true,
            metadata_updated: true,
            runtime_applied: false,
        })
    }

    async fn fetch_url(&self, url: &str) -> Result<Vec<u8>, ProfileStoreError> {
        let https = url.starts_with("https://");
        let loopback_http = is_loopback_http_url(url);
        if !https && !loopback_http {
            return Err(ProfileStoreError::SecurityDenied(
                "only HTTPS or loopback HTTP subscriptions are allowed".into(),
            ));
        }
        let protocol = if https { "=https" } else { "=http" };
        let mut command = Command::new("curl");
        command
            .args([
                "--silent",
                "--show-error",
                "--fail",
                "--location",
                "--max-redirs",
                MAX_REDIRECTS,
                "--proto",
                protocol,
                "--connect-timeout",
                "5",
                "--max-time",
                "20",
                "--max-filesize",
                "524288",
                "--config",
                "-",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command
            .spawn()
            .map_err(|_| ProfileStoreError::Unavailable("subscription request failed".into()))?;
        let escaped_url = curl_config_escape(url)?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(format!("url = \"{}\"\n", escaped_url).as_bytes())
                .await
                .map_err(|_| {
                    ProfileStoreError::Unavailable("subscription request failed".into())
                })?;
        }
        let output = tokio::time::timeout(Duration::from_secs(25), child.wait_with_output())
            .await
            .map_err(|_| ProfileStoreError::Timeout)?
            .map_err(|_| ProfileStoreError::Unavailable("subscription request failed".into()))?;
        if !output.status.success() {
            return Err(ProfileStoreError::Unavailable(
                "subscription request failed".into(),
            ));
        }
        if output.stdout.len() as u64 > MAX_PROFILE_BYTES {
            return Err(ProfileStoreError::SecurityDenied(
                "profile response exceeds the size limit".into(),
            ));
        }
        Ok(output.stdout)
    }

    async fn apply_stored(&self, id: &str) -> Result<ProfileImportResult, ProfileStoreError> {
        let index = self.read_index()?;
        let stored = index
            .profiles
            .iter()
            .find(|p| p.id == id)
            .cloned()
            .ok_or_else(|| ProfileStoreError::NotFound("Luma Profile".into()))?;
        let path = self.source_path(id)?;
        let snapshot = self.register_clash(&stored, &path)?;
        let Some(runtime) = &self.runtime else {
            return Ok(ProfileImportResult {
                summary: Self::to_summary(stored, true),
                source_written: true,
                metadata_updated: true,
                runtime_applied: false,
            });
        };
        if let Err(error) = runtime.apply_profile(&path).await {
            if let Some(snapshot) = snapshot {
                if let Err(rollback) = self.restore_clash(snapshot) {
                    return Err(ProfileStoreError::Conflict(format!(
                        "Mihomo runtime application failed and Clash Verge rollback failed: {rollback}"
                    )));
                }
            }
            return Err(error);
        }
        Ok(ProfileImportResult {
            summary: Self::to_summary(stored, true),
            source_written: true,
            metadata_updated: true,
            runtime_applied: true,
        })
    }

    fn read_current_uid(&self) -> Result<Option<String>, ProfileStoreError> {
        let Some(root) = &self.clash_root else {
            return Ok(None);
        };
        let path = root.join("profiles.yaml");
        if !path.exists() {
            return Ok(None);
        }
        let value = read_yaml_file(&path)?;
        Ok(value
            .get("current")
            .and_then(Value::as_str)
            .map(str::to_string))
    }

    fn register_clash(
        &self,
        stored: &StoredProfile,
        source_path: &Path,
    ) -> Result<Option<ClashSnapshot>, ProfileStoreError> {
        let Some(root) = &self.clash_root else {
            return Ok(None);
        };
        let manifest = root.join("profiles.yaml");
        if !manifest.exists() {
            return Ok(None);
        }
        ensure_contained(root, &manifest)?;
        if fs::symlink_metadata(&manifest)
            .map_err(io_error)?
            .file_type()
            .is_symlink()
        {
            return Err(ProfileStoreError::SecurityDenied(
                "Clash Verge manifest symlinks are not supported".into(),
            ));
        }
        let mut value = read_yaml_file(&manifest)?;
        let Some(map) = value.as_mapping_mut() else {
            return Err(unsupported_schema());
        };
        let Some(items) = map
            .get_mut(Value::String("items".into()))
            .and_then(Value::as_sequence_mut)
        else {
            return Err(unsupported_schema());
        };
        let target_name = format!("{}.yaml", stored.id);
        let mut found = false;
        for item in items.iter_mut() {
            if item.get("uid").and_then(Value::as_str) == Some(stored.id.as_str()) {
                if item.get("type").and_then(Value::as_str) != Some("local") {
                    return Err(unsupported_schema());
                }
                let Some(item_map) = item.as_mapping_mut() else {
                    return Err(unsupported_schema());
                };
                item_map.insert(
                    Value::String("name".into()),
                    Value::String(stored.name.clone()),
                );
                item_map.insert(
                    Value::String("file".into()),
                    Value::String(target_name.clone()),
                );
                item_map.insert(Value::String("updated".into()), Value::Number(now().into()));
                found = true;
            }
        }
        if !found {
            let mut item = Mapping::new();
            item.insert(
                Value::String("uid".into()),
                Value::String(stored.id.clone()),
            );
            item.insert(
                Value::String("name".into()),
                Value::String(stored.name.clone()),
            );
            item.insert(Value::String("type".into()), Value::String("local".into()));
            item.insert(
                Value::String("file".into()),
                Value::String(target_name.clone()),
            );
            item.insert(Value::String("updated".into()), Value::Number(now().into()));
            items.push(Value::Mapping(item));
        }
        map.insert(
            Value::String("current".into()),
            Value::String(stored.id.clone()),
        );
        let target = safe_child(root, &target_name)?;
        if target.exists()
            && fs::symlink_metadata(&target)
                .map_err(io_error)?
                .file_type()
                .is_symlink()
        {
            return Err(ProfileStoreError::SecurityDenied(
                "Clash Verge Profile symlinks are not supported".into(),
            ));
        }
        let bytes = fs::read(source_path).map_err(io_error)?;
        let old_target = fs::read(&target).ok();
        let old_manifest = fs::read(&manifest).map_err(io_error)?;
        atomic_write(&target, &bytes, true)?;
        if let Err(error) = atomic_yaml(&manifest, &value, true) {
            let rollback = restore_file(&target, old_target.as_deref())
                .and_then(|_| atomic_write(&manifest, &old_manifest, false));
            return Err(rollback_failure(error, rollback));
        }
        Ok(Some(ClashSnapshot {
            manifest,
            old_manifest,
            target,
            old_target,
        }))
    }

    fn restore_clash(&self, snapshot: ClashSnapshot) -> Result<(), ProfileStoreError> {
        atomic_write(&snapshot.manifest, &snapshot.old_manifest, false)?;
        match snapshot.old_target {
            Some(bytes) => atomic_write(&snapshot.target, &bytes, false)?,
            None => remove_file_if_exists(&snapshot.target)?,
        }
        Ok(())
    }

    fn unregister_clash(&self, id: &str) -> Result<Option<ClashSnapshot>, ProfileStoreError> {
        let Some(root) = &self.clash_root else {
            return Ok(None);
        };
        let manifest = root.join("profiles.yaml");
        if !manifest.exists() {
            return Ok(None);
        }
        ensure_contained(root, &manifest)?;
        if fs::symlink_metadata(&manifest)
            .map_err(io_error)?
            .file_type()
            .is_symlink()
        {
            return Err(ProfileStoreError::SecurityDenied(
                "Clash Verge manifest symlinks are not supported".into(),
            ));
        }
        let old_manifest = fs::read(&manifest).map_err(io_error)?;
        let mut value = read_yaml_file(&manifest)?;
        let Some(map) = value.as_mapping_mut() else {
            return Err(unsupported_schema());
        };
        let current_is_deleted = map.get("current").and_then(Value::as_str) == Some(id);
        let replacement = {
            let Some(items) = map
                .get_mut(Value::String("items".into()))
                .and_then(Value::as_sequence_mut)
            else {
                return Err(unsupported_schema());
            };
            let Some(index) = items.iter().position(|item| {
                item.get("uid").and_then(Value::as_str) == Some(id)
                    && item.get("type").and_then(Value::as_str) == Some("local")
                    && item.get("file").and_then(Value::as_str) == Some(&format!("{id}.yaml"))
            }) else {
                return Ok(None);
            };
            items.remove(index);
            current_is_deleted
                .then(|| {
                    items.iter().find_map(|item| {
                        let uid = item.get("uid").and_then(Value::as_str)?;
                        let kind = item.get("type").and_then(Value::as_str)?;
                        (!matches!(kind, "script" | "merge")).then_some(uid.to_string())
                    })
                })
                .flatten()
        };
        if current_is_deleted {
            let key = Value::String("current".into());
            match replacement {
                Some(uid) => {
                    map.insert(key, Value::String(uid));
                }
                None => {
                    map.remove(&key);
                }
            }
        }
        let target = safe_child(root, &format!("{id}.yaml"))?;
        if target.exists()
            && fs::symlink_metadata(&target)
                .map_err(io_error)?
                .file_type()
                .is_symlink()
        {
            return Err(ProfileStoreError::SecurityDenied(
                "Clash Verge Profile symlinks are not supported".into(),
            ));
        }
        let old_target = fs::read(&target).ok();
        remove_file_if_exists(&target)?;
        if let Err(error) = atomic_yaml(&manifest, &value, true) {
            let rollback = restore_file(&target, old_target.as_deref())
                .and_then(|_| atomic_write(&manifest, &old_manifest, false));
            return Err(rollback_failure(error, rollback));
        }
        Ok(Some(ClashSnapshot {
            manifest,
            old_manifest,
            target,
            old_target,
        }))
    }

    fn sync_registered_clash(
        &self,
        stored: &StoredProfile,
        source_path: &Path,
    ) -> Result<(), ProfileStoreError> {
        let Some(root) = &self.clash_root else {
            return Ok(());
        };
        let manifest = root.join("profiles.yaml");
        if !manifest.exists() {
            return Ok(());
        }
        let value = read_yaml_file(&manifest)?;
        let Some(items) = value.get("items").and_then(Value::as_sequence) else {
            return Err(unsupported_schema());
        };
        let registered = items.iter().any(|item| {
            item.get("uid").and_then(Value::as_str) == Some(stored.id.as_str())
                && item.get("type").and_then(Value::as_str) == Some("local")
                && item.get("file").and_then(Value::as_str) == Some(&format!("{}.yaml", stored.id))
        });
        if !registered {
            return Ok(());
        }
        let target = safe_child(root, &format!("{}.yaml", stored.id))?;
        if target.exists()
            && fs::symlink_metadata(&target)
                .map_err(io_error)?
                .file_type()
                .is_symlink()
        {
            return Err(ProfileStoreError::SecurityDenied(
                "Clash Verge Profile symlinks are not supported".into(),
            ));
        }
        atomic_write(&target, &fs::read(source_path).map_err(io_error)?, true)
    }
}

#[async_trait]
impl ProfileStorePort for MacProfileStore {
    async fn list_profiles(&self) -> Result<Vec<ProfileSummary>, ProfileStoreError> {
        let current = self.read_current_uid()?;
        let index = self.read_index()?;
        let mut out: Vec<_> = index
            .profiles
            .into_iter()
            .map(|p| Self::to_summary(p.clone(), current.as_deref() == Some(p.id.as_str())))
            .collect();
        if let Some(root) = &self.clash_root {
            let path = root.join("profiles.yaml");
            if path.exists() {
                let value = read_yaml_file(&path)?;
                if let (Some(current), Some(items)) = (
                    value.get("current").and_then(Value::as_str),
                    value.get("items").and_then(Value::as_sequence),
                ) {
                    for item in items {
                        let Some(uid) = item.get("uid").and_then(Value::as_str) else {
                            continue;
                        };
                        if out.iter().any(|p| p.id == uid) {
                            continue;
                        }
                        let kind = item
                            .get("type")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown");
                        let name = item
                            .get("name")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                            .unwrap_or_else(|| format!("Clash Verge {kind}"));
                        let stats = item
                            .get("file")
                            .and_then(Value::as_str)
                            .and_then(|file| read_profile_stats(root, file));
                        let (node_count, group_count, rule_count, metadata_available) = stats
                            .map_or((0, 0, 0, false), |(nodes, groups, rules)| {
                                (nodes, groups, rules, true)
                            });
                        out.push(ProfileSummary {
                            id: uid.to_string(),
                            name,
                            node_count,
                            group_count,
                            rule_count,
                            metadata_available,
                            updated_at: item.get("updated").and_then(Value::as_u64),
                            source: ProfileSource::ClashVerge,
                            owned_by_luma: false,
                            current: uid == current,
                        });
                    }
                }
            }
        }
        out.sort_by_key(|p| (!p.current, p.name.to_lowercase()));
        Ok(out)
    }

    async fn import_subscription(
        &self,
        url: &str,
        suggested_name: Option<&str>,
    ) -> Result<ProfileImportResult, ProfileStoreError> {
        let bytes = self.fetch_url(url).await?;
        self.import_bytes(
            bytes,
            suggested_name,
            ProfileSource::Subscription,
            Some(url),
            None,
        )
        .await
    }

    async fn import_local_file(
        &self,
        path: &Path,
        suggested_name: Option<&str>,
    ) -> Result<ProfileImportResult, ProfileStoreError> {
        let safe = canonical_local_file(path)?;
        let meta = fs::metadata(&safe).map_err(io_error)?;
        if meta.len() > MAX_PROFILE_BYTES {
            return Err(ProfileStoreError::SecurityDenied(
                "local profile exceeds the size limit".into(),
            ));
        }
        self.import_bytes(
            fs::read(safe).map_err(io_error)?,
            suggested_name,
            ProfileSource::LumaLocal,
            None,
            None,
        )
        .await
    }

    async fn use_profile(&self, id: &str) -> Result<ProfileImportResult, ProfileStoreError> {
        self.apply_stored(id).await
    }

    async fn refresh_profile(&self, id: &str) -> Result<ProfileImportResult, ProfileStoreError> {
        let url = self
            .keychain
            .copy_password(&format!("{URL_ACCOUNT_PREFIX}{id}"))
            .await
            .map_err(|_| {
                ProfileStoreError::NotConfigured(
                    "this Profile has no stored subscription source".into(),
                )
            })?;
        let bytes = self.fetch_url(&url).await?;
        let existing = self
            .read_index()?
            .profiles
            .into_iter()
            .find(|p| p.id == id)
            .ok_or_else(|| ProfileStoreError::NotFound("Luma Profile".into()))?;
        self.import_bytes(
            bytes,
            Some(&existing.name),
            ProfileSource::Subscription,
            Some(&url),
            Some(id),
        )
        .await
    }

    async fn delete_profile(&self, id: &str) -> Result<(), ProfileStoreError> {
        let mut index = self.read_index()?;
        if !index.profiles.iter().any(|p| p.id == id) {
            return Err(ProfileStoreError::SecurityDenied(
                "only Luma-owned Profiles can be deleted".into(),
            ));
        }
        let path = self.source_path(id)?;
        let old_source = fs::read(&path).ok();
        let old_index = fs::read(self.index_path()).ok();
        let clash_snapshot = self.unregister_clash(id)?;
        remove_file_if_exists(&path)?;
        index.profiles.retain(|p| p.id != id);
        if let Err(error) = atomic_json(&self.index_path(), &index) {
            let rollback = restore_file(&path, old_source.as_deref())
                .and_then(|_| restore_file(&self.index_path(), old_index.as_deref()))
                .and_then(|_| {
                    clash_snapshot
                        .map(|snapshot| self.restore_clash(snapshot))
                        .unwrap_or(Ok(()))
                });
            return Err(rollback_failure(error, rollback));
        }
        if let Err(error) = self
            .keychain
            .delete(&format!("{URL_ACCOUNT_PREFIX}{id}"))
            .await
        {
            if !matches!(error, KeychainError::NotFound(_)) {
                let rollback = restore_file(&path, old_source.as_deref())
                    .and_then(|_| restore_file(&self.index_path(), old_index.as_deref()))
                    .and_then(|_| {
                        clash_snapshot
                            .map(|snapshot| self.restore_clash(snapshot))
                            .unwrap_or(Ok(()))
                    });
                return Err(rollback_failure(
                    ProfileStoreError::Unavailable(
                        "subscription address could not be removed from Keychain".into(),
                    ),
                    rollback,
                ));
            }
        }
        Ok(())
    }
}

fn normalize_subscription_bytes(bytes: Vec<u8>) -> Result<Vec<u8>, ProfileStoreError> {
    normalize_subscription_bytes_inner(bytes, 0)
}

fn normalize_subscription_bytes_inner(
    bytes: Vec<u8>,
    depth: u8,
) -> Result<Vec<u8>, ProfileStoreError> {
    if depth > 1 || bytes.len() as u64 > MAX_PROFILE_BYTES {
        return Err(ProfileStoreError::SecurityDenied(
            "subscription response exceeds the size limit".into(),
        ));
    }
    if let Ok(text) = String::from_utf8(bytes.clone()) {
        if let Ok(value) = serde_yaml::from_str::<Value>(&text) {
            if value.is_mapping() {
                return Ok(text.into_bytes());
            }
        }
        if let Ok(converted) = convert_node_uris(&text) {
            return Ok(converted);
        }
    }
    let decoded = decode_base64(&bytes).ok_or_else(|| ProfileStoreError::InvalidInput {
        field: "subscription".into(),
        message: "subscription is not Clash YAML or a supported node list".into(),
    })?;
    normalize_subscription_bytes_inner(decoded, depth + 1)
}

fn convert_node_uris(text: &str) -> Result<Vec<u8>, ProfileStoreError> {
    let mut proxies = Vec::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if line.starts_with('#') {
            continue;
        }
        let proxy = if line.starts_with("vless://") {
            parse_vless(line)
        } else if line.starts_with("vmess://") {
            parse_vmess(line)
        } else if line.starts_with("ss://") {
            parse_shadowsocks(line)
        } else if line.starts_with("trojan://") {
            parse_trojan(line)
        } else {
            return Err(ProfileStoreError::InvalidInput {
                field: "subscription".into(),
                message: "subscription contains an unsupported node format".into(),
            });
        };
        proxies.push(proxy?);
        if proxies.len() > 2000 {
            return Err(ProfileStoreError::SecurityDenied(
                "subscription contains too many nodes".into(),
            ));
        }
    }
    if proxies.is_empty() {
        return Err(ProfileStoreError::InvalidInput {
            field: "subscription".into(),
            message: "subscription contains no supported nodes".into(),
        });
    }
    let mut root = Mapping::new();
    root.insert(Value::String("proxies".into()), Value::Sequence(proxies));
    serde_yaml::to_string(&Value::Mapping(root))
        .map(|value| value.into_bytes())
        .map_err(|_| ProfileStoreError::Unavailable("subscription could not be converted".into()))
}

fn parse_vless(uri: &str) -> Result<Value, ProfileStoreError> {
    let (parts, query, name) = parse_uri(uri, "vless")?;
    let mut map = proxy_base("vless", name);
    insert_string(&mut map, "server", parts.host);
    insert_u16(&mut map, "port", parts.port)?;
    insert_string(&mut map, "uuid", parts.user);
    let tls = query_value(query, "security")
        .map(|value| value == "tls" || value == "reality")
        .unwrap_or(false);
    insert_bool(&mut map, "tls", tls);
    if let Some(sni) = query_value(query, "sni") {
        insert_string(&mut map, "servername", sni);
    }
    if let Some(network) = query_value(query, "type") {
        insert_string(&mut map, "network", network);
    }
    Ok(Value::Mapping(map))
}

fn parse_trojan(uri: &str) -> Result<Value, ProfileStoreError> {
    let (parts, query, name) = parse_uri(uri, "trojan")?;
    let mut map = proxy_base("trojan", name);
    insert_string(&mut map, "server", parts.host);
    insert_u16(&mut map, "port", parts.port)?;
    insert_string(&mut map, "password", parts.user);
    insert_bool(&mut map, "tls", true);
    if let Some(sni) = query_value(query, "sni") {
        insert_string(&mut map, "sni", sni);
    }
    Ok(Value::Mapping(map))
}

fn parse_shadowsocks(uri: &str) -> Result<Value, ProfileStoreError> {
    let (body, name) = uri
        .strip_prefix("ss://")
        .and_then(|value| {
            value
                .split_once('#')
                .map_or(Some((value, "")), |(body, name)| Some((body, name)))
        })
        .ok_or_else(|| invalid_uri("ss"))?;
    let body = body.split('?').next().unwrap_or(body);
    let (userinfo, authority) = if let Some((userinfo, authority)) = body.rsplit_once('@') {
        (percent_decode(userinfo), authority.to_string())
    } else {
        let decoded = decode_base64(body.as_bytes()).ok_or_else(|| invalid_uri("ss"))?;
        let decoded = String::from_utf8(decoded).map_err(|_| invalid_uri("ss"))?;
        let (userinfo, authority) = decoded.rsplit_once('@').ok_or_else(|| invalid_uri("ss"))?;
        (userinfo.to_string(), authority.to_string())
    };
    let (host, port) = split_host_port(&authority)?;
    let (cipher, password) = userinfo.split_once(':').ok_or_else(|| invalid_uri("ss"))?;
    let mut map = proxy_base("ss", percent_decode(name));
    insert_string(&mut map, "server", host);
    insert_u16(&mut map, "port", port)?;
    insert_string(&mut map, "cipher", percent_decode(cipher));
    insert_string(&mut map, "password", percent_decode(password));
    Ok(Value::Mapping(map))
}

fn parse_vmess(uri: &str) -> Result<Value, ProfileStoreError> {
    let encoded = uri
        .strip_prefix("vmess://")
        .ok_or_else(|| invalid_uri("vmess"))?;
    let decoded = decode_base64(encoded.as_bytes()).ok_or_else(|| invalid_uri("vmess"))?;
    let value: serde_json::Value =
        serde_json::from_slice(&decoded).map_err(|_| invalid_uri("vmess"))?;
    let mut map = proxy_base(
        "vmess",
        value
            .get("ps")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Imported node"),
    );
    insert_string(
        &mut map,
        "server",
        value
            .get("add")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(""),
    );
    insert_u16(
        &mut map,
        "port",
        value
            .get("port")
            .and_then(serde_json::Value::as_u64)
            .or_else(|| {
                value
                    .get("port")
                    .and_then(serde_json::Value::as_str)
                    .and_then(|v| v.parse().ok())
            }),
    )?;
    insert_string(
        &mut map,
        "uuid",
        value
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(""),
    );
    insert_bool(
        &mut map,
        "tls",
        value.get("tls").and_then(serde_json::Value::as_str) == Some("tls"),
    );
    if let Some(network) = value.get("net").and_then(serde_json::Value::as_str) {
        insert_string(&mut map, "network", network);
    }
    Ok(Value::Mapping(map))
}

fn proxy_base(kind: &str, name: impl Into<String>) -> Mapping {
    let mut map = Mapping::new();
    map.insert(Value::String("name".into()), Value::String(name.into()));
    map.insert(Value::String("type".into()), Value::String(kind.into()));
    map
}

fn insert_string(map: &mut Mapping, key: &str, value: impl Into<String>) {
    map.insert(Value::String(key.into()), Value::String(value.into()));
}

fn insert_bool(map: &mut Mapping, key: &str, value: bool) {
    map.insert(Value::String(key.into()), Value::Bool(value));
}

fn insert_u16(map: &mut Mapping, key: &str, value: Option<u64>) -> Result<(), ProfileStoreError> {
    let Some(value) = value
        .and_then(|value| u16::try_from(value).ok())
        .filter(|value| *value != 0)
    else {
        return Err(invalid_uri("port"));
    };
    map.insert(Value::String(key.into()), Value::Number(value.into()));
    Ok(())
}

struct UriParts {
    user: String,
    host: String,
    port: Option<u64>,
}

fn parse_uri<'a>(
    uri: &'a str,
    scheme: &str,
) -> Result<(UriParts, &'a str, String), ProfileStoreError> {
    let prefix = format!("{scheme}://");
    let value = uri
        .strip_prefix(&prefix)
        .ok_or_else(|| invalid_uri(scheme))?;
    let (authority, query_and_name) = value.split_once('?').unwrap_or((value, ""));
    let (authority, name) = authority
        .split_once('#')
        .map_or((authority, ""), |(authority, name)| (authority, name));
    let (query, fragment) = query_and_name
        .split_once('#')
        .map_or((query_and_name, ""), |(query, name)| (query, name));
    let (userinfo, hostport) = authority
        .rsplit_once('@')
        .ok_or_else(|| invalid_uri(scheme))?;
    let (host, port) = split_host_port(hostport)?;
    Ok((
        UriParts {
            user: percent_decode(userinfo),
            host,
            port,
        },
        query,
        percent_decode(if fragment.is_empty() { name } else { fragment }),
    ))
}

fn split_host_port(value: &str) -> Result<(String, Option<u64>), ProfileStoreError> {
    if let Some(value) = value.strip_prefix('[') {
        let (host, port) = value.split_once(']').ok_or_else(|| invalid_uri("host"))?;
        let port = port.strip_prefix(':').and_then(|value| value.parse().ok());
        return Ok((host.to_string(), port));
    }
    let (host, port) = value
        .rsplit_once(':')
        .map_or((value, None), |(host, port)| {
            (host, port.parse::<u64>().ok())
        });
    Ok((host.to_string(), port))
}

fn query_value(query: &str, key: &str) -> Option<String> {
    query
        .split('&')
        .filter_map(|part| part.split_once('='))
        .find(|(candidate, _)| *candidate == key)
        .map(|(_, value)| percent_decode(value))
}

fn percent_decode(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let (Some(high), Some(low)) =
                (hex_digit(bytes[index + 1]), hex_digit(bytes[index + 2]))
            {
                output.push((high * 16 + low) as char);
                index += 3;
                continue;
            }
        }
        output.push(bytes[index] as char);
        index += 1;
    }
    output
}

fn hex_digit(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn decode_base64(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut output = Vec::new();
    let mut buffer = 0_u32;
    let mut bits = 0_u8;
    for byte in bytes
        .iter()
        .copied()
        .filter(|byte| !byte.is_ascii_whitespace())
    {
        if byte == b'=' {
            break;
        }
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return None,
        };
        buffer = (buffer << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    (!output.is_empty()).then_some(output)
}

fn invalid_uri(kind: &str) -> ProfileStoreError {
    ProfileStoreError::InvalidInput {
        field: "subscription".into(),
        message: format!("invalid {kind} node URI"),
    }
}

fn validate_profile(value: &Value) -> Result<(), ProfileStoreError> {
    let Some(map) = value.as_mapping() else {
        return Err(ProfileStoreError::InvalidInput {
            field: "yaml".into(),
            message: "profile root must be a mapping".into(),
        });
    };
    for key in ["script", "script-providers", "merge", "javascript"] {
        if map.contains_key(Value::String(key.into())) {
            return Err(ProfileStoreError::Unsupported(format!(
                "{key} profiles are not supported"
            )));
        }
    }
    for key in [
        "external-controller",
        "external-controller-unix",
        "secret",
        "authentication",
        "external-ui",
        "listeners",
        "bind-address",
    ] {
        if map.contains_key(Value::String(key.into())) {
            return Err(ProfileStoreError::SecurityDenied(format!(
                "imported Profile cannot configure {key}; Luma keeps controller settings trusted"
            )));
        }
    }
    if map.get("allow-lan").and_then(Value::as_bool) == Some(true) {
        return Err(ProfileStoreError::SecurityDenied(
            "imported Profile cannot enable allow-lan".into(),
        ));
    }
    if let Some(controller) = map.get("external-controller").and_then(Value::as_str) {
        if !controller.starts_with("127.0.0.1:")
            && !controller.starts_with("localhost:")
            && !controller.starts_with("[::1]:")
        {
            return Err(ProfileStoreError::SecurityDenied(
                "imported Profile must use a loopback external-controller".into(),
            ));
        }
    }
    if map
        .get("tun")
        .and_then(|v| v.get("enable"))
        .and_then(Value::as_bool)
        == Some(true)
    {
        return Err(ProfileStoreError::Unsupported(
            "TUN is not supported for imported Profiles".into(),
        ));
    }
    Ok(())
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

fn sequence_len(value: Option<&Value>) -> usize {
    value.and_then(Value::as_sequence).map_or(0, Vec::len)
}
fn safe_name(value: &str) -> Result<String, ProfileStoreError> {
    let value = value.trim();
    if value.is_empty() || value.len() > 120 || value.chars().any(|c| c.is_control()) {
        return Err(ProfileStoreError::InvalidInput {
            field: "name".into(),
            message: "Profile name is invalid".into(),
        });
    }
    Ok(value.to_string())
}
fn valid_id(id: &str) -> bool {
    id.len() == 24 && id.starts_with("p-") && id[2..].chars().all(|c| c.is_ascii_hexdigit())
}
fn new_id(raw: &str, name: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    raw.hash(&mut h);
    name.hash(&mut h);
    now().hash(&mut h);
    format!("p-{:022x}", h.finish())
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
fn io_error(_: std::io::Error) -> ProfileStoreError {
    ProfileStoreError::Unavailable("Profile storage is unavailable".into())
}
fn unsupported_schema() -> ProfileStoreError {
    ProfileStoreError::Unsupported("当前 Clash Verge Profile 结构暂不支持自动写回".into())
}
fn default_clash_root() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| {
        PathBuf::from(h)
            .join("Library/Application Support/io.github.clash-verge-rev.clash-verge-rev")
    })
}

fn safe_child(root: &Path, name: &str) -> Result<PathBuf, ProfileStoreError> {
    if name.contains('/') || name.contains('\\') || name == ".." {
        return Err(ProfileStoreError::SecurityDenied(
            "Profile path escapes its controlled directory".into(),
        ));
    }
    let path = root.join(name);
    ensure_contained(root, &path)?;
    Ok(path)
}

fn read_profile_stats(root: &Path, file: &str) -> Option<(usize, usize, usize)> {
    let path = safe_relative_child(root, file).ok()?;
    let value = read_yaml_file(&path).ok()?;
    Some((
        sequence_len(value.get("proxies")),
        sequence_len(value.get("proxy-groups")),
        sequence_len(value.get("rules")),
    ))
}

fn safe_relative_child(root: &Path, file: &str) -> Result<PathBuf, ProfileStoreError> {
    let relative = Path::new(file);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(ProfileStoreError::SecurityDenied(
            "Clash Verge Profile path escapes its controlled directory".into(),
        ));
    }
    let path = root.join(relative);
    ensure_contained(root, &path)?;
    Ok(path)
}

fn ensure_contained(root: &Path, path: &Path) -> Result<(), ProfileStoreError> {
    let root = root.canonicalize().map_err(|_| {
        ProfileStoreError::SecurityDenied("Profile directory is unavailable".into())
    })?;
    if let Ok(existing) = path.canonicalize() {
        if !existing.starts_with(&root) {
            return Err(ProfileStoreError::SecurityDenied(
                "Profile path escapes its controlled directory".into(),
            ));
        }
    } else if path
        .parent()
        .and_then(|p| p.canonicalize().ok())
        .is_some_and(|p| !p.starts_with(&root))
    {
        return Err(ProfileStoreError::SecurityDenied(
            "Profile path escapes its controlled directory".into(),
        ));
    }
    Ok(())
}
fn canonical_local_file(path: &Path) -> Result<PathBuf, ProfileStoreError> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|_| {
                ProfileStoreError::Unavailable("local Profile path is unavailable".into())
            })?
            .join(path)
    };
    let mut cur = PathBuf::new();
    for c in absolute.components() {
        match c {
            Component::RootDir => cur.push("/"),
            Component::Normal(part) => {
                cur.push(part);
                if fs::symlink_metadata(&cur)
                    .map_err(|_| ProfileStoreError::NotFound("local Profile file".into()))?
                    .file_type()
                    .is_symlink()
                    && !is_system_alias(&cur)
                {
                    return Err(ProfileStoreError::SecurityDenied(
                        "symbolic links are not allowed for local Profiles".into(),
                    ));
                }
            }
            Component::CurDir => {}
            Component::ParentDir => {
                cur.pop();
            }
            Component::Prefix(p) => cur.push(p.as_os_str()),
        }
    }
    let canonical = absolute
        .canonicalize()
        .map_err(|_| ProfileStoreError::NotFound("local Profile file".into()))?;
    if !canonical.is_file() {
        return Err(ProfileStoreError::InvalidInput {
            field: "path".into(),
            message: "local Profile path is not a file".into(),
        });
    }
    Ok(canonical)
}
fn read_yaml_file(path: &Path) -> Result<Value, ProfileStoreError> {
    let meta = fs::metadata(path).map_err(io_error)?;
    if meta.len() > MAX_PROFILE_BYTES {
        return Err(ProfileStoreError::SecurityDenied(
            "Profile metadata exceeds the size limit".into(),
        ));
    }
    let raw = fs::read_to_string(path).map_err(io_error)?;
    serde_yaml::from_str(&raw).map_err(|_| unsupported_schema())
}
fn atomic_write(path: &Path, bytes: &[u8], backup: bool) -> Result<(), ProfileStoreError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_error)?;
    }
    if backup && path.exists() {
        let backup_path = path.with_file_name(format!(
            "{}.bak",
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("profile")
        ));
        fs::copy(path, &backup_path).map_err(io_error)?;
        set_private_file_mode(&backup_path)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes).map_err(io_error)?;
    set_private_file_mode(&tmp)?;
    fs::rename(&tmp, path).map_err(io_error)?;
    set_private_file_mode(path)
}
fn atomic_json<T: Serialize>(path: &Path, value: &T) -> Result<(), ProfileStoreError> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|_| {
        ProfileStoreError::Unavailable("Profile metadata could not be encoded".into())
    })?;
    atomic_write(path, &bytes, true)
}
fn atomic_yaml(path: &Path, value: &Value, backup: bool) -> Result<(), ProfileStoreError> {
    let bytes = serde_yaml::to_string(value).map_err(|_| unsupported_schema())?;
    atomic_write(path, bytes.as_bytes(), backup)
}

fn remove_file_if_exists(path: &Path) -> Result<(), ProfileStoreError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error(error)),
    }
}

fn restore_file(path: &Path, bytes: Option<&[u8]>) -> Result<(), ProfileStoreError> {
    match bytes {
        Some(bytes) => atomic_write(path, bytes, false),
        None => remove_file_if_exists(path),
    }
}

fn rollback_failure(
    original: ProfileStoreError,
    rollback: Result<(), ProfileStoreError>,
) -> ProfileStoreError {
    match rollback {
        Ok(()) => original,
        Err(error) => ProfileStoreError::Conflict(format!("{original}; rollback failed: {error}")),
    }
}

#[cfg(unix)]
fn set_private_file_mode(path: &Path) -> Result<(), ProfileStoreError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(io_error)
}

#[cfg(unix)]
fn set_private_dir_mode(path: &Path) -> Result<(), ProfileStoreError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(io_error)
}

#[cfg(not(unix))]
fn set_private_dir_mode(_path: &Path) -> Result<(), ProfileStoreError> {
    Ok(())
}

#[cfg(not(unix))]
fn set_private_file_mode(_path: &Path) -> Result<(), ProfileStoreError> {
    Ok(())
}

fn is_system_alias(path: &Path) -> bool {
    #[cfg(unix)]
    {
        matches!(path, p if p == Path::new("/tmp") || p == Path::new("/var") || p == Path::new("/etc"))
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use luma_application::{FakeKeychain, ProfileStorePort};
    use std::collections::BTreeMap;

    fn keychain() -> Arc<FakeKeychain> {
        Arc::new(FakeKeychain {
            unlocked: true,
            entries: tokio::sync::Mutex::new(BTreeMap::new()),
        })
    }

    struct FailingRuntime;

    #[async_trait]
    impl RuntimeApplyPort for FailingRuntime {
        async fn apply_profile(&self, _path: &Path) -> Result<(), ProfileStoreError> {
            Err(ProfileStoreError::Conflict(
                "runtime rejected test Profile".into(),
            ))
        }
    }

    #[tokio::test]
    async fn imports_yaml_and_persists_only_redacted_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input.yaml");
        fs::write(&input, "name: Safe\nproxies:\n  - name: node\n    password: token-not-for-ui\nproxy-groups:\n  - name: auto\nrules:\n  - MATCH,DIRECT\n").unwrap();
        let store = MacProfileStore::with_paths(dir.path().join("profiles"), None, keychain());
        let result = store.import_local_file(&input, None).await.unwrap();
        assert_eq!(result.summary.node_count, 1);
        assert_eq!(result.summary.group_count, 1);
        assert_eq!(result.summary.rule_count, 1);
        let raw = fs::read_to_string(dir.path().join("profiles/profiles.json")).unwrap();
        assert!(!raw.contains("token-not-for-ui"));
        assert!(!raw.contains("input.yaml"));
    }

    #[tokio::test]
    async fn rejects_merge_script_and_dangerous_runtime_settings() {
        let dir = tempfile::tempdir().unwrap();
        let store = MacProfileStore::with_paths(dir.path().join("profiles"), None, keychain());
        for (name, yaml) in [
            ("script.yaml", "script: test.js\n"),
            ("merge.yaml", "merge:\n  - x.yaml\n"),
            ("lan.yaml", "allow-lan: true\n"),
            ("tun.yaml", "tun:\n  enable: true\n"),
        ] {
            let path = dir.path().join(name);
            fs::write(&path, yaml).unwrap();
            let error = store.import_local_file(&path, None).await.unwrap_err();
            assert!(!error.to_string().contains(name));
        }
    }

    #[tokio::test]
    async fn local_import_failure_leaves_no_source_file() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("bad.yaml");
        fs::write(&input, "proxies: [").unwrap();
        let store = MacProfileStore::with_paths(dir.path().join("profiles"), None, keychain());
        assert!(store.import_local_file(&input, None).await.is_err());
        assert!(!dir.path().join("profiles").exists());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn rejects_local_symlink_escape() {
        let dir = tempfile::tempdir().unwrap();
        let outside = dir.path().join("outside.yaml");
        let link = dir.path().join("link.yaml");
        fs::write(&outside, "proxies: []").unwrap();
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        let store = MacProfileStore::with_paths(dir.path().join("profiles"), None, keychain());
        assert!(matches!(
            store.import_local_file(&link, None).await,
            Err(ProfileStoreError::SecurityDenied(_))
        ));
    }

    #[tokio::test]
    async fn reads_current_uid_and_writes_only_luma_local_profile() {
        let dir = tempfile::tempdir().unwrap();
        let clash = dir.path().join("clash");
        fs::create_dir_all(&clash).unwrap();
        fs::write(clash.join("profiles.yaml"), "current: old\nitems:\n  - uid: old\n    type: local\n    name: Existing\n    file: old.yaml\n  - uid: Script\n    type: script\n    file: Script.js\n").unwrap();
        let input = dir.path().join("new.yaml");
        fs::write(&input, "proxies: []").unwrap();
        let store = MacProfileStore::with_paths(
            dir.path().join("profiles"),
            Some(clash.clone()),
            keychain(),
        );
        let imported = store
            .import_local_file(&input, Some("New local"))
            .await
            .unwrap();
        let applied = store.use_profile(&imported.summary.id).await.unwrap();
        assert!(!applied.runtime_applied);
        let manifest = fs::read_to_string(clash.join("profiles.yaml")).unwrap();
        assert!(manifest.contains(&imported.summary.id));
        assert!(manifest.contains("Script.js"));
        assert!(manifest.contains("current:"));
        assert!(store.delete_profile(&imported.summary.id).await.is_ok());
        let manifest = fs::read_to_string(clash.join("profiles.yaml")).unwrap();
        assert!(!manifest.contains(&imported.summary.id));
        assert!(!clash.join(format!("{}.yaml", imported.summary.id)).exists());
        assert!(store.delete_profile("Script").await.is_err());
    }

    #[tokio::test]
    async fn runtime_failure_restores_manifest_and_target_file() {
        let dir = tempfile::tempdir().unwrap();
        let clash = dir.path().join("clash");
        fs::create_dir_all(&clash).unwrap();
        fs::write(
            clash.join("profiles.yaml"),
            r#"current: old
items:
  - uid: old
    type: local
    name: Existing
    file: old.yaml
"#,
        )
        .unwrap();
        let input = dir.path().join("new.yaml");
        fs::write(&input, "proxies: []").unwrap();
        let store = MacProfileStore::with_paths(
            dir.path().join("profiles"),
            Some(clash.clone()),
            keychain(),
        )
        .with_runtime(Arc::new(FailingRuntime));
        let imported = store
            .import_local_file(&input, Some("New local"))
            .await
            .unwrap();
        assert!(store.use_profile(&imported.summary.id).await.is_err());
        let manifest = fs::read_to_string(clash.join("profiles.yaml")).unwrap();
        assert!(!manifest.contains(&imported.summary.id));
        assert!(manifest.trim_start().starts_with("current: old"));
        assert!(!clash.join(format!("{}.yaml", imported.summary.id)).exists());
    }

    #[tokio::test]
    async fn refresh_metadata_failure_keeps_previous_source() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("profiles");
        let input = dir.path().join("old.yaml");
        fs::write(&input, "name: Old\nproxies: []").unwrap();
        let store = MacProfileStore::with_paths(root.clone(), None, keychain());
        let imported = store.import_local_file(&input, None).await.unwrap();
        let source = root.join(format!("{}.yaml", imported.summary.id));
        let old = fs::read(&source).unwrap();
        fs::remove_file(root.join("profiles.json")).unwrap();
        fs::create_dir(root.join("profiles.json")).unwrap();
        let error = store
            .import_bytes(
                b"name: New\nproxies: []".to_vec(),
                Some("New"),
                ProfileSource::Subscription,
                None,
                Some(&imported.summary.id),
            )
            .await
            .unwrap_err();
        assert!(!error.to_string().contains("New"));
        assert_eq!(fs::read(source).unwrap(), old);
    }

    #[test]
    fn rejects_controller_and_listener_fields_before_persistence() {
        for key in [
            "external-controller-unix",
            "secret",
            "authentication",
            "external-ui",
            "listeners",
            "bind-address",
        ] {
            let yaml = format!("{key}: forbidden\nproxies: []\n");
            let value: Value = serde_yaml::from_str(&yaml).unwrap();
            assert!(matches!(
                validate_profile(&value),
                Err(ProfileStoreError::SecurityDenied(_))
            ));
        }
    }

    #[test]
    fn loopback_http_validation_accepts_ports_but_not_lookalikes() {
        assert!(is_loopback_http_url("http://127.0.0.1:8080/profile"));
        assert!(is_loopback_http_url("http://localhost:8080/profile"));
        assert!(is_loopback_http_url("http://[::1]:8080/profile"));
        assert!(!is_loopback_http_url("http://127.0.0.1.evil/profile"));
        assert!(!is_loopback_http_url("http://192.168.1.2:8080/profile"));
    }

    #[tokio::test]
    async fn converts_base64_and_common_node_uri_subscriptions() {
        let dir = tempfile::tempdir().unwrap();
        let store = MacProfileStore::with_paths(dir.path().join("profiles"), None, keychain());
        let result = store
            .import_bytes(
                b"vless://11111111-1111-1111-1111-111111111111@example.com:443?security=tls&sni=example.com#Demo".to_vec(),
                None,
                ProfileSource::Subscription,
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(result.summary.node_count, 1);
        let encoded = b"cHJveGllczogW10K".to_vec();
        let converted = normalize_subscription_bytes(encoded).unwrap();
        assert_eq!(String::from_utf8(converted).unwrap(), "proxies: []\n");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn profile_directory_and_files_are_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input.yaml");
        fs::write(&input, "proxies: []").unwrap();
        let root = dir.path().join("profiles");
        let store = MacProfileStore::with_paths(root.clone(), None, keychain());
        let result = store.import_local_file(&input, None).await.unwrap();
        assert_eq!(
            fs::metadata(&root).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(root.join(format!("{}.yaml", result.summary.id)))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}
