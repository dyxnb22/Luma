use crate::ports::{PortMeta, PortsMetaRepoError, PortsMetaRepository};
use async_trait::async_trait;
use luma_storage::PortsMetaStore;
use std::sync::Arc;

pub struct SqlitePortsMetaRepository {
    store: Arc<PortsMetaStore>,
}

impl SqlitePortsMetaRepository {
    pub fn new(store: Arc<PortsMetaStore>) -> Self {
        Self { store }
    }
}

fn map_row(row: luma_storage::PortMetaRow) -> PortMeta {
    PortMeta {
        port: row.port,
        display_name: row.display_name,
        favorite: row.favorite,
        last_seen_at: row.last_seen_at,
        kill_count: row.kill_count,
    }
}

#[async_trait]
impl PortsMetaRepository for SqlitePortsMetaRepository {
    fn list(&self) -> Result<Vec<PortMeta>, PortsMetaRepoError> {
        self.store
            .list()
            .map(|rows| rows.into_iter().map(map_row).collect())
            .map_err(|e| PortsMetaRepoError::msg(e.to_string()))
    }

    fn get(&self, port: u16) -> Result<Option<PortMeta>, PortsMetaRepoError> {
        self.store
            .get(port)
            .map(|opt| opt.map(map_row))
            .map_err(|e| PortsMetaRepoError::msg(e.to_string()))
    }

    fn set_display_name(
        &self,
        port: u16,
        display_name: Option<&str>,
    ) -> Result<(), PortsMetaRepoError> {
        self.store
            .set_display_name(port, display_name)
            .map_err(|e| PortsMetaRepoError::msg(e.to_string()))
    }

    fn set_favorite(&self, port: u16, favorite: bool) -> Result<(), PortsMetaRepoError> {
        self.store
            .set_favorite(port, favorite)
            .map_err(|e| PortsMetaRepoError::msg(e.to_string()))
    }

    fn record_seen(&self, port: u16, seen_at: &str) -> Result<(), PortsMetaRepoError> {
        self.store
            .record_seen(port, seen_at)
            .map_err(|e| PortsMetaRepoError::msg(e.to_string()))
    }

    fn record_kill(&self, port: u16, killed_at: &str) -> Result<(), PortsMetaRepoError> {
        self.store
            .record_kill(port, killed_at)
            .map_err(|e| PortsMetaRepoError::msg(e.to_string()))
    }

    fn delete(&self, port: u16) -> Result<(), PortsMetaRepoError> {
        self.store
            .delete(port)
            .map_err(|e| PortsMetaRepoError::msg(e.to_string()))
    }
}
