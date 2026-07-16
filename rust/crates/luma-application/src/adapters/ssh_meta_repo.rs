use crate::ports::{SshHostMeta, SshMetaRepoError, SshMetaRepository};
use async_trait::async_trait;
use luma_storage::SshMetaStore;
use std::sync::Arc;

pub struct SqliteSshMetaRepository {
    store: Arc<SshMetaStore>,
}

impl SqliteSshMetaRepository {
    pub fn new(store: Arc<SshMetaStore>) -> Self {
        Self { store }
    }
}

fn map_row(row: luma_storage::SshHostMetaRow) -> SshHostMeta {
    SshHostMeta {
        alias: row.alias,
        display_name: row.display_name,
        favorite: row.favorite,
        tags: row.tags,
        last_connected_at: row.last_connected_at,
        connection_count: row.connection_count,
    }
}

#[async_trait]
impl SshMetaRepository for SqliteSshMetaRepository {
    fn list(&self) -> Result<Vec<SshHostMeta>, SshMetaRepoError> {
        self.store
            .list()
            .map(|rows| rows.into_iter().map(map_row).collect())
            .map_err(|e| SshMetaRepoError::msg(e.to_string()))
    }

    fn get(&self, alias: &str) -> Result<Option<SshHostMeta>, SshMetaRepoError> {
        self.store
            .get(alias)
            .map(|opt| opt.map(map_row))
            .map_err(|e| SshMetaRepoError::msg(e.to_string()))
    }

    fn set_favorite(&self, alias: &str, favorite: bool) -> Result<(), SshMetaRepoError> {
        self.store
            .set_favorite(alias, favorite)
            .map_err(|e| SshMetaRepoError::msg(e.to_string()))
    }

    fn record_connection(&self, alias: &str, connected_at: &str) -> Result<(), SshMetaRepoError> {
        self.store
            .record_connection(alias, connected_at)
            .map_err(|e| SshMetaRepoError::msg(e.to_string()))
    }

    fn delete(&self, alias: &str) -> Result<(), SshMetaRepoError> {
        self.store
            .delete(alias)
            .map_err(|e| SshMetaRepoError::msg(e.to_string()))
    }
}
