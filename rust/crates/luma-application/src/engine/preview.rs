pub(crate) const MAX_PREVIEW_BODY_BYTES: usize = 64 * 1024;
const PREVIEW_TRUNCATED_SUFFIX: &str = "\n… [truncated]";

/// UTF-8 safe preview body cap for engine-emitted [`Event::PreviewLoaded`].
pub(crate) fn truncate_preview_body(body: &str) -> String {
    if body.len() <= MAX_PREVIEW_BODY_BYTES {
        return body.to_string();
    }
    let suffix_len = PREVIEW_TRUNCATED_SUFFIX.len();
    let mut end = MAX_PREVIEW_BODY_BYTES.saturating_sub(suffix_len);
    while end > 0 && !body.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{}", &body[..end], PREVIEW_TRUNCATED_SUFFIX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_on_char_boundary() {
        let body = "é".repeat(MAX_PREVIEW_BODY_BYTES);
        let out = truncate_preview_body(&body);
        assert!(out.ends_with(PREVIEW_TRUNCATED_SUFFIX));
        assert!(out.is_char_boundary(out.len()));
    }
}
