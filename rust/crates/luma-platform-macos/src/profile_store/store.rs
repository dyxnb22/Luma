use async_trait::async_trait;
use luma_application::{
    KeychainError, KeychainPort, ProfileImportResult, ProfileSource, ProfileStoreError,
    ProfileStorePort, ProfileSummary,
};
use luma_storage::luma_next_support_dir;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::fs::{
    atomic_json, atomic_write, canonical_local_file, combine_rollbacks, io_error,
    read_optional_file, read_profile_stats, read_yaml_file, remove_file_if_exists, restore_file,
    rollback_failure, safe_child, set_private_dir_mode,
};
use super::parse::{
    new_id, normalize_subscription_bytes, now, safe_name, sequence_len, valid_id, validate_profile,
};
use super::{default_clash_root, MAX_PROFILE_BYTES, URL_ACCOUNT_PREFIX};

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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct StoredProfile {
    pub(super) id: String,
    pub(super) name: String,
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

/// Previous subscription reference kept only while a file/index transaction is in progress.
/// It deliberately has no Debug implementation so a URL cannot be printed accidentally.
#[derive(Clone)]
enum SubscriptionUrlSnapshot {
    Missing,
    Present(String),
}

pub struct MacProfileStore {
    pub(super) root: PathBuf,
    pub(super) clash_root: Option<PathBuf>,
    keychain: Arc<dyn KeychainPort>,
    runtime: Option<Arc<dyn RuntimeApplyPort>>,
    /// Serializes multi-file/Profile-Keychain mutations within one Luma process.
    operation_lock: Mutex<()>,
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
            operation_lock: Mutex::new(()),
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
            operation_lock: Mutex::new(()),
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
    fn subscription_account(id: &str) -> String {
        format!("{URL_ACCOUNT_PREFIX}{id}")
    }
    async fn snapshot_subscription_url(
        &self,
        id: &str,
    ) -> Result<SubscriptionUrlSnapshot, ProfileStoreError> {
        match self
            .keychain
            .copy_password(&Self::subscription_account(id))
            .await
        {
            Ok(url) => Ok(SubscriptionUrlSnapshot::Present(url)),
            Err(KeychainError::NotFound(_)) => Ok(SubscriptionUrlSnapshot::Missing),
            Err(_) => Err(ProfileStoreError::Unavailable(
                "subscription address could not be read from Keychain".into(),
            )),
        }
    }
    async fn restore_subscription_url(
        &self,
        id: &str,
        snapshot: &SubscriptionUrlSnapshot,
    ) -> Result<(), ProfileStoreError> {
        let account = Self::subscription_account(id);
        match snapshot {
            SubscriptionUrlSnapshot::Present(url) => self
                .keychain
                .set_password(&account, url)
                .await
                .map_err(|_| {
                    ProfileStoreError::Unavailable(
                        "subscription address could not be restored in Keychain".into(),
                    )
                }),
            SubscriptionUrlSnapshot::Missing => match self.keychain.delete(&account).await {
                Ok(()) | Err(KeychainError::NotFound(_)) => Ok(()),
                Err(_) => Err(ProfileStoreError::Unavailable(
                    "subscription address could not be restored in Keychain".into(),
                )),
            },
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
        let index_path = self.index_path();
        let old_source = read_optional_file(&source_path)?;
        let old_index = read_optional_file(&index_path)?;
        let previous_index = self.read_index()?;
        let previous_url = match url {
            Some(_) => Some(self.snapshot_subscription_url(&id).await?),
            None => None,
        };
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
        if let Err(error) = atomic_json(&index_path, &index) {
            let rollback = restore_file(&source_path, old_source.as_deref())
                .and_then(|_| restore_file(&index_path, old_index.as_deref()));
            return Err(rollback_failure(error, rollback));
        }
        if let Some(url) = url {
            if self
                .keychain
                .set_password(&Self::subscription_account(&id), url)
                .await
                .is_err()
            {
                let file_rollback = restore_file(&source_path, old_source.as_deref())
                    .and_then(|_| restore_file(&index_path, old_index.as_deref()));
                let keychain_rollback = match previous_url.as_ref() {
                    Some(snapshot) => self.restore_subscription_url(&id, snapshot).await,
                    None => Err(ProfileStoreError::Conflict(
                        "subscription transaction was missing its Keychain snapshot".into(),
                    )),
                };
                return Err(rollback_failure(
                    ProfileStoreError::Unavailable(
                        "subscription address could not be stored in Keychain".into(),
                    ),
                    combine_rollbacks(file_rollback, keychain_rollback),
                ));
            }
        }
        if fixed_id.is_some() {
            if let Err(error) = self.sync_registered_clash(&stored, &source_path) {
                let file_rollback = restore_file(&source_path, old_source.as_deref())
                    .and_then(|_| restore_file(&index_path, old_index.as_deref()));
                let keychain_rollback = match &previous_url {
                    Some(snapshot) => self.restore_subscription_url(&id, snapshot).await,
                    None => Ok(()),
                };
                return Err(rollback_failure(
                    error,
                    combine_rollbacks(file_rollback, keychain_rollback),
                ));
            }
        }
        Ok(ProfileImportResult {
            summary: Self::to_summary(stored, false),
            source_written: true,
            metadata_updated: true,
            runtime_applied: false,
        })
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
}

#[async_trait]
impl ProfileStorePort for MacProfileStore {
    async fn list_profiles(&self) -> Result<Vec<ProfileSummary>, ProfileStoreError> {
        let _operation = self.operation_lock.lock().await;
        let current = self.read_current_uid()?;
        let index = self.read_index()?;
        let mut out: Vec<_> = index
            .profiles
            .into_iter()
            .map(|p| Self::to_summary(p.clone(), current.as_deref() == Some(p.id.as_str())))
            .collect();
        if let Some(root) = &self.clash_root {
            if let Some(path) = self.checked_clash_manifest()? {
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
        let _operation = self.operation_lock.lock().await;
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
        let _operation = self.operation_lock.lock().await;
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
        let _operation = self.operation_lock.lock().await;
        self.apply_stored(id).await
    }

    async fn refresh_profile(&self, id: &str) -> Result<ProfileImportResult, ProfileStoreError> {
        let _operation = self.operation_lock.lock().await;
        let existing = self
            .read_index()?
            .profiles
            .into_iter()
            .find(|p| p.id == id)
            .ok_or_else(|| ProfileStoreError::NotFound("Luma Profile".into()))?;
        if existing.source != "subscription" {
            return Err(ProfileStoreError::NotConfigured(
                "this Profile has no stored subscription source".into(),
            ));
        }
        let url = self
            .keychain
            .copy_password(&Self::subscription_account(id))
            .await
            .map_err(|_| {
                ProfileStoreError::NotConfigured(
                    "this Profile has no stored subscription source".into(),
                )
            })?;
        let bytes = self.fetch_url(&url).await?;
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
        let _operation = self.operation_lock.lock().await;
        let mut index = self.read_index()?;
        let stored = index
            .profiles
            .iter()
            .find(|profile| profile.id == id)
            .cloned()
            .ok_or_else(|| {
                ProfileStoreError::SecurityDenied("only Luma-owned Profiles can be deleted".into())
            })?;
        let path = self.source_path(id)?;
        let index_path = self.index_path();
        let old_source = read_optional_file(&path)?;
        let old_index = read_optional_file(&index_path)?;
        let previous_url = if stored.source == "subscription" {
            Some(self.snapshot_subscription_url(id).await?)
        } else {
            None
        };
        let clash_snapshot = self.unregister_clash(id)?;
        if let Err(error) = remove_file_if_exists(&path) {
            let rollback = clash_snapshot
                .map(|snapshot| self.restore_clash(snapshot))
                .unwrap_or(Ok(()));
            return Err(rollback_failure(error, rollback));
        }
        index.profiles.retain(|p| p.id != id);
        if let Err(error) = atomic_json(&index_path, &index) {
            let files_rollback = restore_file(&path, old_source.as_deref())
                .and_then(|_| restore_file(&index_path, old_index.as_deref()));
            let clash_rollback = clash_snapshot
                .map(|snapshot| self.restore_clash(snapshot))
                .unwrap_or(Ok(()));
            return Err(rollback_failure(
                error,
                combine_rollbacks(files_rollback, clash_rollback),
            ));
        }
        if let Some(previous_url) = previous_url {
            if let Err(error) = self.keychain.delete(&Self::subscription_account(id)).await {
                if !matches!(error, KeychainError::NotFound(_)) {
                    let files_rollback = restore_file(&path, old_source.as_deref())
                        .and_then(|_| restore_file(&index_path, old_index.as_deref()));
                    let clash_rollback = clash_snapshot
                        .map(|snapshot| self.restore_clash(snapshot))
                        .unwrap_or(Ok(()));
                    let keychain_rollback = self.restore_subscription_url(id, &previous_url).await;
                    return Err(rollback_failure(
                        ProfileStoreError::Unavailable(
                            "subscription address could not be removed from Keychain".into(),
                        ),
                        combine_rollbacks(
                            combine_rollbacks(files_rollback, clash_rollback),
                            keychain_rollback,
                        ),
                    ));
                }
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use luma_application::{
        FakeKeychain, KeychainPort, ProfileSource, ProfileStorePort, SecretLabel,
    };
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::Path;
    use std::sync::Arc;

    fn keychain() -> Arc<FakeKeychain> {
        Arc::new(FakeKeychain {
            unlocked: true,
            entries: tokio::sync::Mutex::new(BTreeMap::new()),
        })
    }

    struct FailOnceSetKeychain {
        entries: tokio::sync::Mutex<BTreeMap<String, String>>,
        fail_next_set: tokio::sync::Mutex<bool>,
        fail_next_delete: tokio::sync::Mutex<bool>,
    }

    impl FailOnceSetKeychain {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                entries: tokio::sync::Mutex::new(BTreeMap::new()),
                fail_next_set: tokio::sync::Mutex::new(false),
                fail_next_delete: tokio::sync::Mutex::new(false),
            })
        }

        async fn fail_next_set(&self) {
            *self.fail_next_set.lock().await = true;
        }

        async fn fail_next_delete(&self) {
            *self.fail_next_delete.lock().await = true;
        }
    }

    #[async_trait]
    impl KeychainPort for FailOnceSetKeychain {
        async fn list_labels(&self) -> Result<Vec<SecretLabel>, KeychainError> {
            Ok(self
                .entries
                .lock()
                .await
                .keys()
                .map(|account| SecretLabel {
                    account: account.clone(),
                })
                .collect())
        }

        async fn copy_password(&self, account: &str) -> Result<String, KeychainError> {
            self.entries
                .lock()
                .await
                .get(account)
                .cloned()
                .ok_or_else(|| KeychainError::NotFound(account.into()))
        }

        async fn set_password(&self, account: &str, password: &str) -> Result<(), KeychainError> {
            let mut fail_next_set = self.fail_next_set.lock().await;
            if *fail_next_set {
                *fail_next_set = false;
                return Err(KeychainError::Unavailable("test failure".into()));
            }
            drop(fail_next_set);
            self.entries
                .lock()
                .await
                .insert(account.into(), password.into());
            Ok(())
        }

        async fn delete(&self, account: &str) -> Result<(), KeychainError> {
            let mut fail_next_delete = self.fail_next_delete.lock().await;
            if *fail_next_delete {
                *fail_next_delete = false;
                return Err(KeychainError::Unavailable("test failure".into()));
            }
            drop(fail_next_delete);
            self.entries
                .lock()
                .await
                .remove(account)
                .map(|_| ())
                .ok_or_else(|| KeychainError::NotFound(account.into()))
        }
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
    async fn rejects_merge_script_and_dangerous_runtime_settings_before_persistence() {
        let dir = tempfile::tempdir().unwrap();
        let store = MacProfileStore::with_paths(dir.path().join("profiles"), None, keychain());
        for (name, yaml) in [
            ("script.yaml", "script: test.js\n"),
            ("merge.yaml", "merge:\n  - x.yaml\n"),
            ("lan.yaml", "allow-lan: true\n"),
            ("tun.yaml", "tun:\n  enable: true\n"),
            ("port.yaml", "port: 7890\nproxies: []\n"),
            ("mixed.yaml", "mixed-port: 7890\nproxies: []\n"),
            ("socks.yaml", "socks-port: 7891\nproxies: []\n"),
            ("dns.yaml", "dns:\n  enable: true\nproxies: []\n"),
            ("mode.yaml", "mode: global\nproxies: []\n"),
            (
                "controller.yaml",
                "external-controller: 127.0.0.1:9090\nproxies: []\n",
            ),
        ] {
            let path = dir.path().join(name);
            fs::write(&path, yaml).unwrap();
            let error = store.import_local_file(&path, None).await.unwrap_err();
            assert!(!error.to_string().contains(name));
        }
        assert!(!dir.path().join("profiles").exists());
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

    #[tokio::test]
    async fn subscription_update_rolls_back_files_and_keychain_after_keychain_failure() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("profiles");
        let keychain = FailOnceSetKeychain::new();
        let store = MacProfileStore::with_paths(root.clone(), None, keychain.clone());
        let old_url = "https://example.invalid/old";
        let imported = store
            .import_bytes(
                b"name: Old\nproxies: []\n".to_vec(),
                None,
                ProfileSource::Subscription,
                Some(old_url),
                None,
            )
            .await
            .unwrap();
        let id = imported.summary.id;
        let source_path = root.join(format!("{id}.yaml"));
        let index_path = root.join("profiles.json");
        let old_source = fs::read(&source_path).unwrap();
        let old_index = fs::read(&index_path).unwrap();
        keychain.fail_next_set().await;

        let error = store
            .import_bytes(
                b"name: New\nproxies: []\n".to_vec(),
                None,
                ProfileSource::Subscription,
                Some("https://example.invalid/new"),
                Some(&id),
            )
            .await
            .unwrap_err();

        assert!(!error.to_string().contains("example.invalid"));
        assert_eq!(fs::read(source_path).unwrap(), old_source);
        assert_eq!(fs::read(index_path).unwrap(), old_index);
        assert_eq!(
            keychain
                .copy_password(&MacProfileStore::subscription_account(&id))
                .await
                .unwrap(),
            old_url
        );
    }

    #[tokio::test]
    async fn subscription_refresh_sync_failure_restores_files_and_keychain() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("profiles");
        let clash = dir.path().join("clash");
        let keychain = keychain();
        let store =
            MacProfileStore::with_paths(root.clone(), Some(clash.clone()), keychain.clone());
        let old_url = "https://example.invalid/old";
        let imported = store
            .import_bytes(
                b"name: Old\nproxies: []\n".to_vec(),
                None,
                ProfileSource::Subscription,
                Some(old_url),
                None,
            )
            .await
            .unwrap();
        let id = imported.summary.id;
        let source_path = root.join(format!("{id}.yaml"));
        let index_path = root.join("profiles.json");
        let old_source = fs::read(&source_path).unwrap();
        let old_index = fs::read(&index_path).unwrap();
        fs::create_dir_all(&clash).unwrap();
        let manifest = clash.join("profiles.yaml");
        fs::write(
            &manifest,
            format!(
                "current: {id}\nitems:\n  - uid: {id}\n    type: local\n    name: Old\n    file: {id}.yaml\n"
            ),
        )
        .unwrap();
        let old_manifest = fs::read(&manifest).unwrap();
        // A directory at the managed target makes the fake Clash sync fail before it can write.
        fs::create_dir(clash.join(format!("{id}.yaml"))).unwrap();

        let error = store
            .import_bytes(
                b"name: New\nproxies: []\n".to_vec(),
                None,
                ProfileSource::Subscription,
                Some("https://example.invalid/new"),
                Some(&id),
            )
            .await
            .unwrap_err();

        assert!(!error.to_string().contains("example.invalid"));
        assert_eq!(fs::read(source_path).unwrap(), old_source);
        assert_eq!(fs::read(index_path).unwrap(), old_index);
        assert_eq!(fs::read(manifest).unwrap(), old_manifest);
        assert_eq!(
            keychain
                .copy_password(&MacProfileStore::subscription_account(&id))
                .await
                .unwrap(),
            old_url
        );
    }

    #[tokio::test]
    async fn subscription_delete_keychain_failure_restores_files_clash_and_url() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("profiles");
        let clash = dir.path().join("clash");
        let keychain = FailOnceSetKeychain::new();
        let store =
            MacProfileStore::with_paths(root.clone(), Some(clash.clone()), keychain.clone());
        let old_url = "https://example.invalid/old";
        let imported = store
            .import_bytes(
                b"name: Old\nproxies: []\n".to_vec(),
                None,
                ProfileSource::Subscription,
                Some(old_url),
                None,
            )
            .await
            .unwrap();
        let id = imported.summary.id;
        let source_path = root.join(format!("{id}.yaml"));
        let index_path = root.join("profiles.json");
        let old_source = fs::read(&source_path).unwrap();
        let old_index = fs::read(&index_path).unwrap();
        fs::create_dir_all(&clash).unwrap();
        let manifest = clash.join("profiles.yaml");
        fs::write(
            &manifest,
            format!(
                "current: {id}\nitems:\n  - uid: {id}\n    type: local\n    name: Old\n    file: {id}.yaml\n"
            ),
        )
        .unwrap();
        let target = clash.join(format!("{id}.yaml"));
        fs::write(&target, b"old target").unwrap();
        let old_manifest = fs::read(&manifest).unwrap();
        let old_target = fs::read(&target).unwrap();
        keychain.fail_next_delete().await;

        let error = store.delete_profile(&id).await.unwrap_err();

        assert!(!error.to_string().contains("example.invalid"));
        assert_eq!(fs::read(source_path).unwrap(), old_source);
        assert_eq!(fs::read(index_path).unwrap(), old_index);
        assert_eq!(fs::read(manifest).unwrap(), old_manifest);
        assert_eq!(fs::read(target).unwrap(), old_target);
        assert_eq!(
            keychain
                .copy_password(&MacProfileStore::subscription_account(&id))
                .await
                .unwrap(),
            old_url
        );
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn list_profiles_rejects_a_symlinked_clash_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let clash = dir.path().join("clash");
        fs::create_dir_all(&clash).unwrap();
        let safe_target = clash.join("other.yaml");
        fs::write(&safe_target, "current: old\nitems: []\n").unwrap();
        std::os::unix::fs::symlink(&safe_target, clash.join("profiles.yaml")).unwrap();
        let store =
            MacProfileStore::with_paths(dir.path().join("profiles"), Some(clash), keychain());
        assert!(matches!(
            store.list_profiles().await,
            Err(ProfileStoreError::SecurityDenied(_))
        ));
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
