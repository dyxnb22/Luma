use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TranslationError {
    #[error("translation unavailable: {0}")]
    Unavailable(String),
    #[error("empty input")]
    EmptyInput,
}

#[derive(Clone, Debug)]
pub struct TranslationResult {
    pub source_text: String,
    pub translated_text: String,
    pub target_language: String,
}

#[async_trait]
pub trait TranslatorPort: Send + Sync {
    async fn translate(
        &self,
        text: &str,
        target_language: &str,
    ) -> Result<TranslationResult, TranslationError>;
}
