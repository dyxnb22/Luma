use std::hash::{Hash, Hasher};

pub(super) fn opaque_component(value: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Avoid exposing obvious credential-shaped values that a provider may have put into a label.
pub(super) fn redact_label(value: &str) -> String {
    if value.contains("://") || looks_like_uuid(value) {
        return "[redacted]".into();
    }
    value.to_string()
}

pub(super) fn looks_like_uuid(value: &str) -> bool {
    const UUID_LEN: usize = 36;
    if value.len() < UUID_LEN {
        return false;
    }
    (0..=value.len() - UUID_LEN).any(|start| {
        let Some(candidate) = value.get(start..start + UUID_LEN) else {
            return false;
        };
        let groups: Vec<_> = candidate.split('-').collect();
        groups.len() == 5
            && [8, 4, 4, 4, 12]
                .iter()
                .zip(groups)
                .all(|(want, got)| got.len() == *want && got.chars().all(|c| c.is_ascii_hexdigit()))
    })
}
