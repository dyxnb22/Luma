use crate::ports::{ResumeContextsRepository, ResumeRepoError};
use async_trait::async_trait;
use luma_storage::{ResumeContext, ResumeStore};
use std::sync::Arc;

pub struct JsonResumeContextsRepository {
    store: Arc<ResumeStore>,
}

impl JsonResumeContextsRepository {
    pub fn new(store: Arc<ResumeStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl ResumeContextsRepository for JsonResumeContextsRepository {
    fn list(&self) -> Result<Vec<ResumeContext>, ResumeRepoError> {
        self.store.list().map_err(ResumeRepoError::from_store)
    }

    fn get(&self, name: &str) -> Result<Option<ResumeContext>, ResumeRepoError> {
        self.store.get(name).map_err(ResumeRepoError::from_store)
    }

    fn upsert(&self, context: ResumeContext) -> Result<ResumeContext, ResumeRepoError> {
        self.store
            .upsert(context)
            .map_err(ResumeRepoError::from_store)
    }

    fn delete(&self, name: &str) -> Result<(), ResumeRepoError> {
        self.store.delete(name).map_err(ResumeRepoError::from_store)
    }

    fn mark_resumed(&self, name: &str) -> Result<ResumeContext, ResumeRepoError> {
        self.store
            .mark_resumed(name)
            .map_err(ResumeRepoError::from_store)
    }

    fn rebuild_empty(&self) -> Result<(), ResumeRepoError> {
        self.store
            .rebuild_empty()
            .map_err(ResumeRepoError::from_store)
    }
}
