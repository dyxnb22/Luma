//! macOS `/usr/bin/say` adapter. Do **not** use in automated tests.

use async_trait::async_trait;
use luma_application::{SpeechAccent, SpeechError, SpeechPort};

pub struct MacSpeech;

#[async_trait]
impl SpeechPort for MacSpeech {
    async fn speak(&self, text: &str, accent: SpeechAccent) -> Result<(), SpeechError> {
        let text = text.trim();
        if text.is_empty() {
            return Ok(());
        }
        if accent == SpeechAccent::Uk {
            // Prefer common UK voices; fall back to default if missing.
            for voice in ["Daniel", "Kate", "Serena", "Oliver"] {
                let status = tokio::process::Command::new("/usr/bin/say")
                    .args(["-v", voice, text])
                    .status()
                    .await
                    .map_err(|e| SpeechError::Failed(e.to_string()))?;
                if status.success() {
                    return Ok(());
                }
            }
        }
        let status = tokio::process::Command::new("/usr/bin/say")
            .arg(text)
            .status()
            .await
            .map_err(|e| SpeechError::Failed(e.to_string()))?;
        if status.success() {
            Ok(())
        } else {
            Err(SpeechError::Failed(format!("say exited {status}")))
        }
    }
}
