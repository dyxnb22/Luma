use async_trait::async_trait;
use luma_storage::{new_blank_context, ResumeContext, ResumeStoreError};
use std::sync::Mutex;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{0}")]
pub struct ResumeRepoError(pub String);

impl ResumeRepoError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn from_store(err: ResumeStoreError) -> Self {
        Self(err.to_string())
    }

    pub fn is_corrupt(&self) -> bool {
        self.0.contains("corrupt resume store")
    }
}

#[async_trait]
pub trait ResumeContextsRepository: Send + Sync {
    fn list(&self) -> Result<Vec<ResumeContext>, ResumeRepoError>;
    fn get(&self, name: &str) -> Result<Option<ResumeContext>, ResumeRepoError>;
    fn upsert(&self, context: ResumeContext) -> Result<ResumeContext, ResumeRepoError>;
    fn delete(&self, name: &str) -> Result<(), ResumeRepoError>;
    fn mark_resumed(&self, name: &str) -> Result<ResumeContext, ResumeRepoError>;
    fn rebuild_empty(&self) -> Result<(), ResumeRepoError>;
}

/// In-memory repository for tests.
#[derive(Default)]
pub struct MemoryResumeContextsRepository {
    inner: Mutex<Vec<ResumeContext>>,
    pub fail_list: Mutex<Option<String>>,
}

impl MemoryResumeContextsRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_contexts(contexts: Vec<ResumeContext>) -> Self {
        Self {
            inner: Mutex::new(contexts),
            fail_list: Mutex::new(None),
        }
    }

    pub fn fail_with(&self, message: impl Into<String>) {
        *self.fail_list.lock().expect("lock") = Some(message.into());
    }
}

#[async_trait]
impl ResumeContextsRepository for MemoryResumeContextsRepository {
    fn list(&self) -> Result<Vec<ResumeContext>, ResumeRepoError> {
        if let Some(msg) = self.fail_list.lock().expect("lock").clone() {
            return Err(ResumeRepoError::msg(msg));
        }
        let mut contexts = self.inner.lock().expect("lock").clone();
        luma_storage::sort_contexts_by_recency(&mut contexts);
        Ok(contexts)
    }

    fn get(&self, name: &str) -> Result<Option<ResumeContext>, ResumeRepoError> {
        let key = luma_storage::normalize_name(name).map_err(ResumeRepoError::from_store)?;
        Ok(self
            .inner
            .lock()
            .expect("lock")
            .iter()
            .find(|c| c.name == key)
            .cloned())
    }

    fn upsert(&self, context: ResumeContext) -> Result<ResumeContext, ResumeRepoError> {
        let key = luma_storage::normalize_name(&context.name).map_err(ResumeRepoError::from_store)?;
        let mut guard = self.inner.lock().expect("lock");
        let mut saved = context;
        saved.name = key.clone();
        if saved.display_name.trim().is_empty() {
            saved.display_name = key.clone();
        }
        let now = luma_storage::resume_now_iso();
        if let Some(existing) = guard.iter_mut().find(|c| c.name == key) {
            saved.created_at = existing.created_at.clone();
            if saved.updated_at.is_empty() {
                saved.updated_at = now;
            }
            *existing = saved.clone();
        } else {
            if saved.created_at.is_empty() {
                saved.created_at = now.clone();
            }
            if saved.updated_at.is_empty() {
                saved.updated_at = now;
            }
            guard.push(saved.clone());
        }
        Ok(saved)
    }

    fn delete(&self, name: &str) -> Result<(), ResumeRepoError> {
        let key = luma_storage::normalize_name(name).map_err(ResumeRepoError::from_store)?;
        let mut guard = self.inner.lock().expect("lock");
        let before = guard.len();
        guard.retain(|c| c.name != key);
        if guard.len() == before {
            return Err(ResumeRepoError::msg(format!("not found: {key}")));
        }
        Ok(())
    }

    fn mark_resumed(&self, name: &str) -> Result<ResumeContext, ResumeRepoError> {
        let key = luma_storage::normalize_name(name).map_err(ResumeRepoError::from_store)?;
        let mut guard = self.inner.lock().expect("lock");
        let Some(ctx) = guard.iter_mut().find(|c| c.name == key) else {
            return Err(ResumeRepoError::msg(format!("not found: {key}")));
        };
        let now = luma_storage::resume_now_iso();
        ctx.last_resumed_at = Some(now.clone());
        ctx.updated_at = now;
        Ok(ctx.clone())
    }

    fn rebuild_empty(&self) -> Result<(), ResumeRepoError> {
        *self.fail_list.lock().expect("lock") = None;
        self.inner.lock().expect("lock").clear();
        Ok(())
    }
}

pub fn blank_context(name: &str) -> Result<ResumeContext, ResumeRepoError> {
    new_blank_context(name).map_err(ResumeRepoError::from_store)
}
