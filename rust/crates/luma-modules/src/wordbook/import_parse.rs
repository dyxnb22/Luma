//! Parse CSV / Markdown / ChatGPT-style word lists into content rows.

use luma_application::WordContentInput;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedImport {
    pub rows: Vec<WordContentInput>,
    pub skipped: usize,
}

pub fn parse_csv(text: &str) -> ParsedImport {
    let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty());
    let Some(header_line) = lines.next() else {
        return ParsedImport {
            rows: Vec::new(),
            skipped: 0,
        };
    };
    let headers: Vec<String> = split_csv_line(header_line)
        .into_iter()
        .map(|h| h.trim().to_lowercase())
        .collect();
    let term_idx = find_header(&headers, &["term", "word", "单词"]);
    let Some(term_idx) = term_idx else {
        return ParsedImport {
            rows: Vec::new(),
            skipped: 0,
        };
    };
    let phonetic_idx = find_header(&headers, &["phonetic", "音标"]);
    let meaning_idx = find_header(&headers, &["meaning", "definition", "中文", "释义"]);
    let example_idx = find_header(&headers, &["example", "sentence", "例句"]);
    let category_idx = find_header(&headers, &["category", "tag", "分类"]);

    let mut rows = Vec::new();
    let mut skipped = 0;
    for line in lines {
        let cells = split_csv_line(line);
        let term = cells.get(term_idx).map(|s| s.trim()).unwrap_or("");
        if term.is_empty() {
            skipped += 1;
            continue;
        }
        rows.push(WordContentInput {
            term: term.into(),
            phonetic: cell_at(&cells, phonetic_idx),
            meaning: cell_at(&cells, meaning_idx),
            example: cell_at(&cells, example_idx),
            category: cell_at(&cells, category_idx),
        });
    }
    ParsedImport { rows, skipped }
}

pub fn parse_text(text: &str) -> ParsedImport {
    if text.contains('|') {
        return parse_markdown_table(text);
    }
    parse_dash_lines(text)
}

fn parse_markdown_table(text: &str) -> ParsedImport {
    let mut headers: Vec<String> = Vec::new();
    let mut rows = Vec::new();
    let mut skipped = 0;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("```") {
            continue;
        }
        if !line.starts_with('|') {
            continue;
        }
        let cells: Vec<String> = line
            .split('|')
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .map(str::to_string)
            .collect();
        if cells.is_empty() {
            continue;
        }
        let lower = cells.join("|").to_lowercase();
        if lower.contains("---") {
            continue;
        }
        if lower.contains("word") || lower.contains("term") || lower.contains("单词") {
            headers = cells.iter().map(|c| c.to_lowercase()).collect();
            continue;
        }
        if cells.len() < 2 {
            skipped += 1;
            continue;
        }
        let term = if headers.is_empty() {
            cells[0].clone()
        } else {
            cell_for(&headers, &cells, &["word", "term", "单词"])
        };
        if term.trim().is_empty() {
            skipped += 1;
            continue;
        }
        let phonetic = if headers.is_empty() {
            String::new()
        } else {
            cell_for(&headers, &cells, &["phonetic", "音标"])
        };
        let meaning = if headers.is_empty() {
            cells.get(1).cloned().unwrap_or_default()
        } else {
            cell_for(&headers, &cells, &["meaning", "definition", "释义", "中文"])
        };
        let example = if headers.is_empty() {
            cells.get(2).cloned().unwrap_or_default()
        } else {
            cell_for(&headers, &cells, &["example", "sentence", "例句"])
        };
        let category = if headers.is_empty() {
            cells.get(3).cloned().unwrap_or_default()
        } else {
            cell_for(&headers, &cells, &["category", "tag", "分类"])
        };
        rows.push(WordContentInput {
            term: term.trim().into(),
            phonetic,
            meaning,
            example,
            category,
        });
    }
    ParsedImport { rows, skipped }
}

fn parse_dash_lines(text: &str) -> ParsedImport {
    let mut rows = Vec::new();
    let mut skipped = 0;
    for raw in text.lines() {
        let mut line = raw.trim().to_string();
        if line.is_empty() {
            continue;
        }
        // Strip list prefixes: 1. / - / * / 1、
        if let Some(rest) = strip_list_prefix(&line) {
            line = rest;
        }
        let parts: Vec<&str> = if line.contains(" - ") {
            line.splitn(3, " - ").collect()
        } else if line.contains('：') {
            line.splitn(3, '：').collect()
        } else if line.contains(':') {
            line.splitn(3, ':').collect()
        } else {
            skipped += 1;
            continue;
        };
        if parts.len() < 2 {
            skipped += 1;
            continue;
        }
        let term = parts[0].trim();
        if term.is_empty() {
            skipped += 1;
            continue;
        }
        rows.push(WordContentInput {
            term: term.into(),
            phonetic: String::new(),
            meaning: parts[1].trim().into(),
            example: parts
                .get(2)
                .map(|s| s.trim().to_string())
                .unwrap_or_default(),
            category: String::new(),
        });
    }
    ParsedImport { rows, skipped }
}

fn strip_list_prefix(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let bytes = trimmed.as_bytes();
    if bytes.first() == Some(&b'-') || bytes.first() == Some(&b'*') {
        return Some(trimmed[1..].trim_start().to_string());
    }
    let mut i = 0;
    while i < trimmed.len() && trimmed.as_bytes()[i].is_ascii_digit() {
        i += 1;
    }
    if i > 0 {
        let rest = &trimmed[i..];
        if rest.starts_with('.') || rest.starts_with(')') || rest.starts_with('、') {
            return Some(rest[1..].trim_start().to_string());
        }
    }
    None
}

fn split_csv_line(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    cur.push('"');
                    chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    out.push(cur.trim().to_string());
    out
}

fn find_header(headers: &[String], names: &[&str]) -> Option<usize> {
    headers.iter().position(|h| names.iter().any(|n| h == n))
}

fn cell_at(cells: &[String], idx: Option<usize>) -> String {
    idx.and_then(|i| cells.get(i).cloned())
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn cell_for(headers: &[String], cells: &[String], names: &[&str]) -> String {
    for name in names {
        if let Some(idx) = headers.iter().position(|h| h == name || h == *name) {
            if let Some(v) = cells.get(idx) {
                return v.trim().to_string();
            }
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_csv_with_english_headers() {
        let text = "term,phonetic,meaning,example,category\nlatency,/x/,延迟,High latency,System\n";
        let parsed = parse_csv(text);
        assert_eq!(parsed.rows.len(), 1);
        assert_eq!(parsed.rows[0].term, "latency");
        assert_eq!(parsed.rows[0].meaning, "延迟");
    }

    #[test]
    fn parses_dash_lines() {
        let text =
            "1. latency - 延迟 - High latency can make an app feel slow.\nthroughput - 吞吐量\n";
        let parsed = parse_text(text);
        assert_eq!(parsed.rows.len(), 2);
        assert_eq!(parsed.rows[0].term, "latency");
        assert!(parsed.rows[0].example.contains("High latency"));
    }

    #[test]
    fn parses_markdown_table() {
        let text = r#"
| word | meaning | example | category |
| --- | --- | --- | --- |
| embedding | 嵌入向量 | The model creates an embedding. | AI |
"#;
        let parsed = parse_text(text);
        assert_eq!(parsed.rows.len(), 1);
        assert_eq!(parsed.rows[0].term, "embedding");
        assert_eq!(parsed.rows[0].category, "AI");
    }
}
