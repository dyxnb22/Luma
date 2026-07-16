use luma_application::ProfileStoreError;
use serde_yaml::{Mapping, Value};
use std::fs;
use std::path::{Path, PathBuf};

use super::fs::{
    atomic_write, atomic_yaml, ensure_contained, io_error, read_optional_file, read_yaml_file,
    remove_file_if_exists, restore_file, rollback_failure, safe_child, unsupported_schema,
};
use super::parse::now;
use super::store::{MacProfileStore, StoredProfile};

pub(super) struct ClashSnapshot {
    pub(super) manifest: PathBuf,
    pub(super) old_manifest: Vec<u8>,
    pub(super) target: PathBuf,
    pub(super) old_target: Option<Vec<u8>>,
}

impl MacProfileStore {
    pub(super) fn read_current_uid(&self) -> Result<Option<String>, ProfileStoreError> {
        let Some(path) = self.checked_clash_manifest()? else {
            return Ok(None);
        };
        let value = read_yaml_file(&path)?;
        Ok(value
            .get("current")
            .and_then(Value::as_str)
            .map(str::to_string))
    }
    pub(super) fn checked_clash_manifest(&self) -> Result<Option<PathBuf>, ProfileStoreError> {
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
        Ok(Some(manifest))
    }
    pub(super) fn register_clash(
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
        let old_target = read_optional_file(&target)?;
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
    pub(super) fn restore_clash(&self, snapshot: ClashSnapshot) -> Result<(), ProfileStoreError> {
        atomic_write(&snapshot.manifest, &snapshot.old_manifest, false)?;
        match snapshot.old_target {
            Some(bytes) => atomic_write(&snapshot.target, &bytes, false)?,
            None => remove_file_if_exists(&snapshot.target)?,
        }
        Ok(())
    }
    pub(super) fn unregister_clash(
        &self,
        id: &str,
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
        let old_target = read_optional_file(&target)?;
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
    pub(super) fn sync_registered_clash(
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
        let old_target = read_optional_file(&target)?;
        let bytes = fs::read(source_path).map_err(io_error)?;
        if let Err(error) = atomic_write(&target, &bytes, true) {
            let rollback = restore_file(&target, old_target.as_deref());
            return Err(rollback_failure(error, rollback));
        }
        Ok(())
    }
}
