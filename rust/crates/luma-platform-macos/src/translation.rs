//! Compatibility-only translator adapters (no production module registered).
//! System Translation typically needs an app host; CLI reports Unavailable.
//! Kept for port/adapter tests — not wired in `bins/luma` compose.

use async_trait::async_trait;

pub use luma_application::{TranslationError, TranslationResult, TranslatorPort as Translator};

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
