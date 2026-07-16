use super::SshModule;
use luma_application::ResolvedSshHost;
use std::collections::HashMap;

impl SshModule {
    pub(super) async fn refresh(&self) {
        let aliases = match self.config.list_aliases() {
            Ok(aliases) => {
                *self.aliases.write().await = aliases.clone();
                aliases
            }
            Err(_) => {
                *self.aliases.write().await = Vec::new();
                Vec::new()
            }
        };
        {
            let mut resolved = self.resolved_cache.write().await;
            resolved.retain(|alias, _| aliases.iter().any(|a| a == alias));
        }
        if let Some(meta) = &self.meta {
            match meta.list() {
                Ok(rows) => {
                    *self.meta_error.write().await = None;
                    let mut map = HashMap::new();
                    for row in rows {
                        map.insert(row.alias.clone(), row);
                    }
                    *self.meta_cache.write().await = map;
                }
                Err(err) => {
                    *self.meta_error.write().await = Some(err.to_string());
                    *self.meta_cache.write().await = HashMap::new();
                }
            }
        }
    }

    pub(super) async fn resolve_host(&self, alias: &str) -> Option<ResolvedSshHost> {
        if let Some(cached) = self.resolved_cache.read().await.get(alias).cloned() {
            return Some(cached);
        }
        match self.config.resolve(alias) {
            Ok(host) => {
                self.resolved_cache
                    .write()
                    .await
                    .insert(alias.to_string(), host.clone());
                Some(host)
            }
            Err(_) => None,
        }
    }

    pub(super) async fn alias_is_known(&self, alias: &str) -> bool {
        let cached = self.aliases.read().await;
        if !cached.is_empty() {
            return cached.iter().any(|a| a == alias);
        }
        drop(cached);
        self.config
            .list_aliases()
            .ok()
            .is_some_and(|aliases| aliases.iter().any(|a| a == alias))
    }

    pub(super) async fn favorite_for_alias(&self, alias: &str) -> bool {
        self.meta_cache
            .read()
            .await
            .get(alias)
            .map(|m| m.favorite)
            .unwrap_or(false)
    }
}
