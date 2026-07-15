//! Parse Records Markdown tables into structured rows.

use sha2::{Digest, Sha256};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ParsedRecordRow {
    pub name: String,
    pub rating: Option<i64>,
    pub note: String,
    pub source_key: String,
    pub line_no: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Serialize)]
pub struct ParseFileReport {
    pub category_name: String,
    pub source_file: String,
    pub rows: Vec<ParsedRecordRow>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub file_hash: String,
}

/// Normalize a record name for uniqueness checks.
pub fn normalize_record_name(name: &str) -> String {
    name.split_whitespace()
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

pub fn hash_file_content(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Stable identity for a row within a source file. It intentionally does not use
/// the line number so inserting another Markdown row does not rename every row
/// below it.
pub fn record_source_key(source_file: &str, name: &str) -> String {
    format!("{source_file}:{}", normalize_record_name(name))
}

/// Hash the imported row content, independent of its line number or file-wide hash.
pub fn hash_record_content(row: &ParsedRecordRow) -> String {
    let mut hasher = Sha256::new();
    hasher.update(row.source_key.as_bytes());
    hasher.update([0]);
    hasher.update(row.name.as_bytes());
    hasher.update([0]);
    if let Some(rating) = row.rating {
        hasher.update(rating.to_string().as_bytes());
    }
    hasher.update([0]);
    hasher.update(row.note.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
pub fn parse_markdown_file(
    source_file: &str,
    category_name: &str,
    bytes: &[u8],
) -> ParseFileReport {
    parse_markdown_file_with_identity(source_file, source_file, category_name, bytes)
}

/// Parse a file while keeping its user-facing name separate from the stable
/// identity used by import idempotency. The latter may include the canonical
/// import root, so two roots containing `电影.md` do not collide.
pub fn parse_markdown_file_with_identity(
    source_file: &str,
    source_identity: &str,
    category_name: &str,
    bytes: &[u8],
) -> ParseFileReport {
    let file_hash = hash_file_content(bytes);
    let text = match std::str::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => {
            return ParseFileReport {
                category_name: category_name.into(),
                source_file: source_file.into(),
                file_hash,
                errors: vec![format!("{source_file}: invalid UTF-8")],
                ..Default::default()
            };
        }
    };
    let mut report = ParseFileReport {
        category_name: category_name.into(),
        source_file: source_file.into(),
        file_hash,
        ..Default::default()
    };
    let mut headers: Vec<String> = Vec::new();
    let mut seen_names = std::collections::HashSet::new();
    let mut in_fence = false;
    for (idx, raw) in text.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw.trim();
        if line.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence || line.is_empty() {
            continue;
        }
        if !line.starts_with('|') {
            continue;
        }
        let cells: Vec<String> = split_table_cells(line);
        if cells.is_empty() {
            continue;
        }
        if is_separator_row(&cells) {
            continue;
        }
        if headers.is_empty() && is_header_row(&cells) {
            headers = cells.iter().map(|c| c.trim().to_lowercase()).collect();
            if name_column_index(&headers).is_none() {
                report.warnings.push(format!(
                    "{source_file}:{line_no}: table header missing name column; using positional columns"
                ));
            }
            continue;
        }
        let (name, rating, note, row_warnings) =
            parse_data_row(&headers, &cells, source_file, line_no);
        for w in row_warnings {
            report.warnings.push(w);
        }
        let name = name.trim();
        if name.is_empty() {
            report
                .warnings
                .push(format!("{source_file}:{line_no}: empty name skipped"));
            continue;
        }
        let norm = normalize_record_name(name);
        if !seen_names.insert(norm.clone()) {
            report.warnings.push(format!(
                "{source_file}:{line_no}: duplicate name \"{name}\" skipped"
            ));
            continue;
        }
        let source_key = record_source_key(source_identity, name);
        report.rows.push(ParsedRecordRow {
            name: name.to_string(),
            rating,
            note,
            source_key,
            line_no,
        });
    }
    if headers.is_empty() && !report.rows.is_empty() {
        report.warnings.push(format!(
            "{source_file}: no recognized table header; parsed rows positionally"
        ));
    }
    report
}

fn split_table_cells(line: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' && chars.peek() == Some(&'|') {
            current.push(chars.next().expect("peeked pipe"));
        } else if ch == '|' {
            cells.push(strip_markdown_link(current.trim()));
            current.clear();
        } else {
            current.push(ch);
        }
    }
    cells.push(strip_markdown_link(current.trim()));

    if line.trim_start().starts_with('|') && cells.first().is_some_and(String::is_empty) {
        cells.remove(0);
    }
    if line.trim_end().ends_with('|') && cells.last().is_some_and(String::is_empty) {
        cells.pop();
    }
    cells
}

fn strip_markdown_link(cell: &str) -> String {
    let trimmed = cell.trim();
    if let Some(inner) = trimmed.strip_prefix('[') {
        if let Some(mid) = inner.find("](") {
            if inner.ends_with(')') && mid > 0 {
                return inner[..mid].to_string();
            }
        }
    }
    trimmed.to_string()
}

fn is_separator_row(cells: &[String]) -> bool {
    !cells.is_empty()
        && cells.iter().all(|cell| {
            let cell = cell.trim();
            !cell.is_empty() && cell.chars().all(|ch| ch == '-' || ch == ':')
        })
}

fn is_header_row(cells: &[String]) -> bool {
    let lower: Vec<String> = cells.iter().map(|c| c.to_lowercase()).collect();
    lower
        .iter()
        .filter(|c| {
            matches!(
                c.as_str(),
                "名字"
                    | "名称"
                    | "name"
                    | "title"
                    | "评分"
                    | "rating"
                    | "score"
                    | "备注"
                    | "note"
                    | "notes"
                    | "comment"
            )
        })
        .count()
        >= 2
}

fn name_column_index(headers: &[String]) -> Option<usize> {
    find_header(headers, &["名字", "名称", "name", "title"])
}

fn rating_column_index(headers: &[String]) -> Option<usize> {
    find_header(headers, &["评分", "rating", "score"])
}

fn note_column_index(headers: &[String]) -> Option<usize> {
    find_header(headers, &["备注", "note", "notes", "comment"])
}

fn find_header(headers: &[String], names: &[&str]) -> Option<usize> {
    headers.iter().position(|h| names.iter().any(|n| h == n))
}

fn parse_data_row(
    headers: &[String],
    cells: &[String],
    source_file: &str,
    line_no: usize,
) -> (String, Option<i64>, String, Vec<String>) {
    let mut warnings = Vec::new();
    let name_idx = name_column_index(headers);
    let rating_idx = rating_column_index(headers);
    let note_idx = note_column_index(headers);

    let name = if let Some(i) = name_idx {
        cells.get(i).cloned().unwrap_or_default()
    } else {
        cells.first().cloned().unwrap_or_default()
    };

    let rating_raw = if let Some(i) = rating_idx {
        cells.get(i).map(|s| s.as_str()).unwrap_or("")
    } else if headers.is_empty() {
        cells.get(1).map(|s| s.as_str()).unwrap_or("")
    } else {
        ""
    };

    let note = if let Some(i) = note_idx {
        cells.get(i).cloned().unwrap_or_default()
    } else if headers.is_empty() {
        cells.get(2).cloned().unwrap_or_default()
    } else {
        String::new()
    };

    if headers.len() > 3 && cells.len() > 3 {
        warnings.push(format!("{source_file}:{line_no}: extra columns ignored"));
    }
    if headers.len() >= 2 && cells.len() < 2 && !name.trim().is_empty() {
        warnings.push(format!("{source_file}:{line_no}: missing columns"));
    }

    let rating = parse_rating(rating_raw, source_file, line_no, &mut warnings);
    (name, rating, note, warnings)
}

fn parse_rating(
    raw: &str,
    source_file: &str,
    line_no: usize,
    warnings: &mut Vec<String>,
) -> Option<i64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let Ok(value) = trimmed.parse::<i64>() else {
        warnings.push(format!(
            "{source_file}:{line_no}: non-numeric rating \"{trimmed}\" ignored"
        ));
        return None;
    };
    if !(1..=10).contains(&value) {
        warnings.push(format!(
            "{source_file}:{line_no}: rating {value} out of range 1-10 ignored"
        ));
        return None;
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_table() {
        let md = r#"# 电影
| 名字 | 评分 | 备注 |
|---|---:|---|
| 沙丘 | 8 | 史诗 |
| 星际穿越 | 9 | [诺兰](https://x) |
"#;
        let report = parse_markdown_file("电影.md", "电影", md.as_bytes());
        assert_eq!(report.rows.len(), 2);
        assert_eq!(report.rows[0].name, "沙丘");
        assert_eq!(report.rows[0].rating, Some(8));
        assert_eq!(report.rows[1].name, "星际穿越");
        assert_eq!(report.rows[1].note, "诺兰");
    }

    #[test]
    fn empty_table_ok() {
        let md = "# 电影\n\n| 名字 | 评分 | 备注 |\n|---|---:|---|\n";
        let report = parse_markdown_file("电影.md", "电影", md.as_bytes());
        assert!(report.rows.is_empty());
        assert!(report.errors.is_empty());
    }

    #[test]
    fn invalid_utf8_is_error() {
        let report = parse_markdown_file("电影.md", "电影", &[0xff, 0xfe]);
        assert!(!report.errors.is_empty());
    }

    #[test]
    fn duplicate_name_skipped() {
        let md = "| 名字 | 评分 | 备注 |\n|---|---:|---|\n| A | 1 | |\n| A | 2 | |\n";
        let report = parse_markdown_file("x.md", "x", md.as_bytes());
        assert_eq!(report.rows.len(), 1);
        assert!(report.warnings.iter().any(|w| w.contains("duplicate")));
    }

    #[test]
    fn preserves_empty_middle_cells() {
        let md = "| 名字 | 评分 | 备注 |\n|---|---:|---|\n| A | | 备注 |\n";
        let report = parse_markdown_file("x.md", "x", md.as_bytes());
        assert_eq!(report.rows.len(), 1);
        assert_eq!(report.rows[0].rating, None);
        assert_eq!(report.rows[0].note, "备注");
    }

    #[test]
    fn keeps_non_link_bracket_text_and_strips_full_link() {
        let md = "| 名字 | 评分 | 备注 |\n|---|---:|---|\n| A | 8 | 推荐 [诺兰](https://x) |\n";
        let report = parse_markdown_file("x.md", "x", md.as_bytes());
        assert_eq!(report.rows[0].note, "推荐 [诺兰](https://x)");

        let md = "| 名字 | 评分 | 备注 |\n|---|---:|---|\n| A | 8 | [诺兰](https://x) |\n";
        let report = parse_markdown_file("x.md", "x", md.as_bytes());
        assert_eq!(report.rows[0].note, "诺兰");
    }

    #[test]
    fn ignores_table_looking_lines_inside_fences() {
        let md = "```md\n| A | 8 | code |\n```\n| 名字 | 评分 | 备注 |\n|---|---:|---|\n| B | 7 | real |\n";
        let report = parse_markdown_file("x.md", "x", md.as_bytes());
        assert_eq!(report.rows.len(), 1);
        assert_eq!(report.rows[0].name, "B");
    }

    #[test]
    fn positional_table_row_is_not_mistaken_for_header() {
        let md = "| A | 8 | 备注 |\n|---|---:|---|\n| B | 7 | 第二行 |\n";
        let report = parse_markdown_file("x.md", "x", md.as_bytes());
        assert_eq!(report.rows.len(), 2);
        assert_eq!(report.rows[0].name, "A");
        assert_eq!(report.rows[0].note, "备注");
    }

    #[test]
    fn bad_rating_warning() {
        let md = "| 名字 | 评分 | 备注 |\n|---|---:|---|\n| A | abc | |\n| B | 11 | |\n";
        let report = parse_markdown_file("x.md", "x", md.as_bytes());
        assert_eq!(report.rows.len(), 2);
        assert!(report.rows[0].rating.is_none());
        assert!(report.rows[1].rating.is_none());
        assert!(report.warnings.len() >= 2);
    }

    #[test]
    fn does_not_extract_digits_from_non_numeric_rating_text() {
        let md = "| 名字 | 评分 | 备注 |\n|---|---:|---|\n| A | score 8 | |\n";
        let report = parse_markdown_file("x.md", "x", md.as_bytes());
        assert_eq!(report.rows[0].rating, None);
        assert!(report.warnings.iter().any(|w| w.contains("score 8")));
    }

    #[test]
    fn source_identity_can_be_distinct_from_display_name() {
        let md = "| 名字 | 评分 | 备注 |\n|---|---:|---|\n| A | 8 | |\n";
        let report =
            parse_markdown_file_with_identity("电影.md", "/one/电影.md", "电影", md.as_bytes());
        assert_eq!(report.source_file, "电影.md");
        assert_eq!(report.rows[0].source_key, "/one/电影.md:A");
    }
}
