//! Markdown frontmatter and content extraction for notes indexing.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrontmatterResult {
    pub has_frontmatter: bool,
    pub yaml: String,
    pub body: String,
    pub warning: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagsResult {
    pub tags: Vec<String>,
    pub warning: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TitleResult {
    pub title: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LinkKind {
    Internal,
    External,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtractedLink {
    pub raw_href: String,
    pub target_path: Option<String>,
    pub kind: LinkKind,
}

/// Split YAML frontmatter delimited by `---` lines at the start of the document.
pub fn split_frontmatter(content: &[u8]) -> FrontmatterResult {
    let text = match std::str::from_utf8(content) {
        Ok(s) => s,
        Err(_) => {
            return FrontmatterResult {
                has_frontmatter: false,
                yaml: String::new(),
                body: String::from_utf8_lossy(content).into_owned(),
                warning: Some("invalid UTF-8".into()),
            };
        }
    };

    if !text.starts_with("---") {
        return FrontmatterResult {
            has_frontmatter: false,
            yaml: String::new(),
            body: text.to_string(),
            warning: None,
        };
    }

    let after_open = &text[3..];
    let close = after_open.find("\n---");
    let Some(close_idx) = close else {
        return FrontmatterResult {
            has_frontmatter: false,
            yaml: String::new(),
            body: text.to_string(),
            warning: None,
        };
    };

    let yaml = &after_open[..close_idx];
    let body_start = close_idx + 4; // \n---
    let body = if body_start < after_open.len() {
        after_open[body_start..]
            .trim_start_matches('\n')
            .to_string()
    } else {
        String::new()
    };

    FrontmatterResult {
        has_frontmatter: true,
        yaml: yaml.trim_start_matches('\n').to_string(),
        body,
        warning: None,
    }
}

/// Extract `tags:` from YAML frontmatter (string or sequence).
pub fn extract_tags(content: &[u8]) -> TagsResult {
    let fm = split_frontmatter(content);
    if !fm.has_frontmatter {
        return TagsResult {
            tags: Vec::new(),
            warning: fm.warning,
        };
    }

    let yaml_value: Result<serde_yaml::Value, _> = serde_yaml::from_str(&fm.yaml);
    let Ok(value) = yaml_value else {
        return TagsResult {
            tags: Vec::new(),
            warning: Some("invalid YAML in frontmatter".into()),
        };
    };

    let Some(tags_val) = value.get("tags") else {
        return TagsResult {
            tags: Vec::new(),
            warning: fm.warning,
        };
    };

    let mut tags = Vec::new();
    match tags_val {
        serde_yaml::Value::String(s) => {
            let t = s.trim();
            if !t.is_empty() {
                tags.push(t.to_string());
            }
        }
        serde_yaml::Value::Sequence(seq) => {
            for item in seq {
                if let serde_yaml::Value::String(s) = item {
                    let t = s.trim();
                    if !t.is_empty() {
                        tags.push(t.to_string());
                    }
                }
            }
        }
        _ => {}
    }

    dedupe_tags(&mut tags);
    TagsResult {
        tags,
        warning: fm.warning,
    }
}

fn dedupe_tags(tags: &mut Vec<String>) {
    let mut seen = std::collections::HashSet::new();
    tags.retain(|t| seen.insert(t.clone()));
}

/// First ATX or Setext heading in the markdown body; falls back to file stem.
pub fn extract_title(content: &[u8], file_name: &str) -> TitleResult {
    let fm = split_frontmatter(content);
    let body = fm.body;

    if let Some(title) = first_heading(&body) {
        return TitleResult { title };
    }

    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(file_name);
    TitleResult {
        title: stem.to_string(),
    }
}

fn first_heading(body: &str) -> Option<String> {
    let options = Options::empty();
    let parser = Parser::new_ext(body, options);
    let mut in_h = false;
    let mut text = String::new();
    for event in parser {
        match event {
            Event::Start(Tag::Heading { .. }) => {
                in_h = true;
                text.clear();
            }
            Event::End(TagEnd::Heading(_)) if in_h => {
                let t = text.trim().to_string();
                if !t.is_empty() {
                    return Some(t);
                }
                in_h = false;
            }
            Event::Text(cow) if in_h => text.push_str(&cow),
            Event::Code(cow) if in_h => text.push_str(&cow),
            _ => {}
        }
    }
    None
}

/// Plain text from markdown body (Text events joined with spaces).
pub fn extract_body(content: &[u8]) -> String {
    let text = match std::str::from_utf8(content) {
        Ok(s) => s,
        Err(_) => return String::new(),
    };
    let fm = split_frontmatter(text.as_bytes());
    let options = Options::empty();
    let parser = Parser::new_ext(&fm.body, options);
    let mut parts = Vec::new();
    for event in parser {
        if let Event::Text(cow) = event {
            let t = cow.trim();
            if !t.is_empty() {
                parts.push(t.to_string());
            }
        }
    }
    parts.join(" ")
}

/// Extract links from markdown, resolving internal paths relative to `source_rel`.
pub fn extract_links(content: &[u8], source_rel: &str) -> Vec<ExtractedLink> {
    let text = match std::str::from_utf8(content) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let fm = split_frontmatter(text.as_bytes());
    let options = Options::empty();
    let parser = Parser::new_ext(&fm.body, options);
    let mut links = Vec::new();
    let mut in_link = false;
    let mut current_href = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::Link { dest_url, .. }) => {
                in_link = true;
                current_href = dest_url.to_string();
            }
            Event::End(TagEnd::Link) => {
                if in_link {
                    if let Some(link) = parse_href(&current_href, source_rel) {
                        links.push(link);
                    }
                }
                in_link = false;
                current_href.clear();
            }
            _ => {}
        }
    }

    dedupe_links(&mut links);
    links
}

fn dedupe_links(links: &mut Vec<ExtractedLink>) {
    let mut seen = std::collections::HashSet::new();
    links.retain(|link| {
        let key = (
            link.raw_href.clone(),
            link.kind,
            link.target_path.clone().unwrap_or_default(),
        );
        seen.insert(key)
    });
}

fn parse_href(raw: &str, source_rel: &str) -> Option<ExtractedLink> {
    let href = raw.trim();
    if href.is_empty() || href.starts_with('#') {
        return None;
    }

    let lower = href.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("mailto:")
    {
        return Some(ExtractedLink {
            raw_href: href.to_string(),
            target_path: None,
            kind: LinkKind::External,
        });
    }

    let (path_part, _fragment) = match href.split_once('#') {
        Some((p, f)) if !p.is_empty() => (p, f),
        _ => (href, ""),
    };

    let resolved = resolve_internal_path(path_part, source_rel)?;
    Some(ExtractedLink {
        raw_href: href.to_string(),
        target_path: Some(resolved),
        kind: LinkKind::Internal,
    })
}

fn resolve_internal_path(href: &str, source_rel: &str) -> Option<String> {
    let href = href.trim();
    if href.is_empty() {
        return None;
    }

    let mut path = if href.starts_with('/') {
        href.trim_start_matches('/').to_string()
    } else {
        let source_dir = Path::new(source_rel)
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        source_dir.join(href).to_string_lossy().replace('\\', "/")
    };

    path = normalize_path(&path)?;

    if !path.to_ascii_lowercase().ends_with(".md") {
        path.push_str(".md");
    }

    Some(path)
}

fn normalize_path(path: &str) -> Option<String> {
    let parts: Vec<&str> = path
        .split('/')
        .filter(|p| !p.is_empty() && *p != ".")
        .collect();
    let mut stack: Vec<&str> = Vec::new();
    for part in parts {
        if part == ".." {
            if stack.is_empty() {
                return None;
            }
            stack.pop();
        } else {
            stack.push(part);
        }
    }
    if stack.is_empty() {
        return Some(String::new());
    }
    Some(stack.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn fixture(rel: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/notes-workspaces")
            .join(rel)
    }

    #[test]
    fn tags_demo_extracts_tags() {
        let content = fs::read(fixture("basic/tags-demo.md")).unwrap();
        let result = extract_tags(&content);
        assert_eq!(result.tags, vec!["alpha", "beta"]);
        assert!(result.warning.is_none());
    }

    #[test]
    fn broken_frontmatter_warning() {
        let content = fs::read(fixture("basic/broken-frontmatter.md")).unwrap();
        let tags = extract_tags(&content);
        assert!(tags.tags.is_empty());
        assert!(tags.warning.is_some());
        let title = extract_title(&content, "broken-frontmatter.md");
        assert_eq!(title.title, "Broken Frontmatter");
    }

    #[test]
    fn title_from_heading_or_stem() {
        let readme = fs::read(fixture("basic/readme.md")).unwrap();
        assert_eq!(extract_title(&readme, "readme.md").title, "readme");
        let tags = fs::read(fixture("basic/tags-demo.md")).unwrap();
        assert_eq!(extract_title(&tags, "tags-demo.md").title, "Tags Demo");
        let link = fs::read(fixture("basic/link-source.md")).unwrap();
        assert_eq!(extract_title(&link, "link-source.md").title, "Link Source");
    }

    #[test]
    fn links_skip_fragment_only() {
        let content = b"# Title\n\nSee [section](#section) and [empty](#).\n";
        let links = extract_links(content, "doc.md");
        assert!(
            links.is_empty(),
            "fragment-only hrefs must be skipped: {links:?}"
        );
    }

    #[test]
    fn links_dedupe_identical_hrefs() {
        let content = b"[first](target.md)\n[second](target.md)\n";
        let links = extract_links(content, "doc.md");
        assert_eq!(
            links.len(),
            1,
            "duplicate hrefs from one source must dedupe"
        );
    }

    #[test]
    fn links_internal_and_external() {
        let content = fs::read(fixture("basic/link-source.md")).unwrap();
        let links = extract_links(&content, "link-source.md");
        assert!(links.iter().any(|l| {
            l.kind == LinkKind::Internal && l.target_path.as_deref() == Some("link-target.md")
        }));
        assert!(links.iter().any(|l| l.kind == LinkKind::External));
        assert!(links.iter().any(|l| {
            l.kind == LinkKind::Internal && l.target_path.as_deref() == Some("does-not-exist.md")
        }));
    }

    #[test]
    fn split_frontmatter_unclosed() {
        let content = b"---\ntags: foo\nno closing";
        let fm = split_frontmatter(content);
        assert!(!fm.has_frontmatter);
    }
}
