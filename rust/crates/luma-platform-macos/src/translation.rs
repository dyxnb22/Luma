//! System Translation capability port.
//! Apple's Translation framework is ObjC/framework-hosted and typically needs an app host.
//! CLI adapter reports structured Unavailable; FakeTranslation enables module tests.

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
pub trait Translator: Send + Sync {
    async fn translate(
        &self,
        text: &str,
        target_language: &str,
    ) -> Result<TranslationResult, TranslationError>;
}

pub struct MacTranslator;

#[async_trait]
impl Translator for MacTranslator {
    async fn translate(
        &self,
        text: &str,
        target_language: &str,
    ) -> Result<TranslationResult, TranslationError> {
        if text.trim().is_empty() {
            return Err(TranslationError::EmptyInput);
        }
        let _ = target_language;
        Err(TranslationError::Unavailable(
            "Translation framework is not callable from pure Rust CLI without an app-host bridge"
                .into(),
        ))
    }
}

/// Offline fake: prefixes text so modules can map success vs failure.
pub struct FakeTranslator {
    pub available: bool,
}

#[async_trait]
impl Translator for FakeTranslator {
    async fn translate(
        &self,
        text: &str,
        target_language: &str,
    ) -> Result<TranslationResult, TranslationError> {
        if text.trim().is_empty() {
            return Err(TranslationError::EmptyInput);
        }
        if !self.available {
            return Err(TranslationError::Unavailable("fake offline".into()));
        }
        Ok(TranslationResult {
            source_text: text.to_string(),
            translated_text: format!("[{target_language}] {text}"),
            target_language: target_language.to_string(),
        })
    }
}
