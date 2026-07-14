//! Heuristics for values that must not enter searchable clipboard history.
//! Prefer false positives (skip capture) over storing high-confidence secrets.

/// Privacy heuristic shared by clipboard capture and import (not a substitute for suppression leases).
pub fn looks_secret(text: &str) -> bool {
    if text.len() > 20_000 {
        return true;
    }
    let lower = text.to_lowercase();
    if lower.contains("password")
        || lower.contains("secret")
        || lower.contains("api_key")
        || lower.contains("-----begin ")
    {
        return true;
    }
    if looks_like_jwt(text) {
        return true;
    }
    if lower.contains("bearer ") || lower.contains("authorization:") {
        return true;
    }
    if lower.contains("token=") || lower.contains("token:") {
        return true;
    }
    // OpenAI-style secret keys
    if text.contains("sk-") {
        return true;
    }
    // GitHub PATs / fine-grained / OAuth / user-to-server
    if text.contains("ghp_")
        || text.contains("gho_")
        || text.contains("ghu_")
        || text.contains("ghs_")
        || text.contains("ghr_")
    {
        return true;
    }
    // AWS access key id
    if text.contains("AKIA") {
        return true;
    }
    false
}

fn looks_like_jwt(text: &str) -> bool {
    let trimmed = text.trim();
    let parts: Vec<&str> = trimmed.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    parts.iter().all(|p| {
        !p.is_empty()
            && p.len() >= 8
            && p.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keywords_and_pem() {
        assert!(looks_secret("password=x"));
        assert!(looks_secret("-----BEGIN PRIVATE KEY-----\nabc"));
        assert!(!looks_secret("hello world"));
    }

    #[test]
    fn high_confidence_tokens() {
        assert!(looks_secret(
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.signature"
        ));
        assert!(looks_secret("Bearer eyJhbGciOiJIUzI1NiJ9.abc.def"));
        assert!(looks_secret("Authorization: Bearer xyz"));
        assert!(looks_secret("https://example.com?token=abc123"));
        assert!(looks_secret("sk-proj-abcdefghijklmnopqrstuvwxyz"));
        assert!(looks_secret("ghp_abcdefghijklmnopqrstuvwxyz012345"));
        assert!(looks_secret("AKIAabcdefghijklmnopqrst"));
    }
}
