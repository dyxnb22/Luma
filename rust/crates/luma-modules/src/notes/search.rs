use super::rows::{status_row, unavailable_row};
use super::NotesModule;
use luma_application::{
    NotesDirectoryEntryKind, NotesScanStatusView, NotesWorkspaceError, SearchSink,
};
use luma_domain::Query;
use luma_protocol::SearchItemDto;
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

const BROWSE_MAX_ENTRIES: usize = 500;

fn humanize_issue_type(issue_type: &str) -> &'static str {
    match issue_type {
        "unreadable" => "Couldn't index",
        "oversized" => "File too large",
        "frontmatter_warning" => "Frontmatter warning",
        "symlink_skipped" => "Skipped symlink",
        "walk_error" => "Folder unreadable",
        _ => "Index issue",
    }
}

fn short_issue_message(message: &str) -> String {
    let trimmed = message.trim();
    let cleaned = trimmed
        .strip_prefix("sqlite: ")
        .or_else(|| trimmed.strip_prefix("sqlite:"))
        .unwrap_or(trimmed);
    if cleaned.chars().count() > 120 {
        let mut out: String = cleaned.chars().take(117).collect();
        out.push('…');
        out
    } else {
        cleaned.to_string()
    }
}

fn scan_status_needs_row(status: &NotesScanStatusView) -> bool {
    match status {
        NotesScanStatusView::Running { .. } | NotesScanStatusView::Failed { .. } => true,
        NotesScanStatusView::Completed { errors, .. } => *errors > 0,
        NotesScanStatusView::Idle => false,
    }
}

fn browse_argument<'a>(rest_raw: &'a str, rest_check: &str) -> Option<&'a str> {
    if rest_check.is_empty() || rest_check == "browse" {
        return Some("");
    }
    let trimmed = rest_raw.trim();
    trimmed
        .strip_prefix("browse")
        .or_else(|| trimmed.strip_prefix("Browse"))
        .or_else(|| trimmed.strip_prefix("ls"))
        .or_else(|| trimmed.strip_prefix("LS"))
        .map(str::trim)
}

impl NotesModule {
    pub(super) async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        let root = self.root.read().await.clone();
        let Some(root) = root else {
            self.emit_results(
                &sink,
                vec![SearchItemDto {
                    id: "notes:configure".into(),
                    module_id: "luma.notes".into(),
                    title: "Choose a Notes root folder".into(),
                    subtitle: Some("Run: luma config set --notes-root ~/Notes".into()),
                    kind: "not_configured".into(),
                    score: 0.0,
                    primary_action_id: "seed_config".into(),
                    primary_action_label: "Show command".into(),
                    ..Default::default()
                }],
                vec![],
            )
            .await;
            return;
        };

        let rest = query.rest_normalized();
        if rest == "new" {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0);
            let path = root.join("Inbox").join(format!("note-{stamp}.md"));
            self.emit_results(
                &sink,
                vec![SearchItemDto {
                    id: format!("note:{}", path.display()),
                    module_id: "luma.notes".into(),
                    title: "Create note".into(),
                    subtitle: Some(path.display().to_string()),
                    kind: "create".into(),
                    score: 100.0,
                    primary_action_id: "create".into(),
                    primary_action_label: "Create".into(),
                    ..Default::default()
                }],
                vec![],
            )
            .await;
            return;
        }

        if rest == "daily" || rest == "today" {
            let date = match self.clock.today_ymd() {
                Ok(date) => date,
                Err(error) => {
                    self.emit_results(
                        &sink,
                        vec![SearchItemDto {
                            id: "note:clock-error".into(),
                            module_id: "luma.notes".into(),
                            title: "Daily note unavailable".into(),
                            subtitle: Some(crate::ux::friendly_store_error(&error.to_string())),
                            kind: "unavailable".into(),
                            score: 0.0,
                            primary_action_id: "noop".into(),
                            primary_action_label: "Unavailable".into(),
                            ..Default::default()
                        }],
                        vec![],
                    )
                    .await;
                    return;
                }
            };
            let candidate = root.join("Daily").join(format!("{date}.md"));
            let existing = match self
                .workspace
                .existing_note(root.clone(), candidate.clone(), cancel.clone())
                .await
            {
                Ok(existing) => existing,
                Err(NotesWorkspaceError::Cancelled) => return,
                Err(error) => {
                    self.emit_results(
                        &sink,
                        vec![unavailable_row("Daily note unavailable", error.to_string())],
                        vec![],
                    )
                    .await;
                    return;
                }
            };
            let (path, exists) = match existing {
                Some(path) => (path.path, true),
                None => (candidate, false),
            };
            self.emit_results(
                &sink,
                vec![SearchItemDto {
                    id: format!("note:{}", path.display()),
                    module_id: "luma.notes".into(),
                    title: if exists {
                        format!("Open daily note ({date})")
                    } else {
                        format!("Create daily note ({date})")
                    },
                    subtitle: Some(path.display().to_string()),
                    kind: if exists { "note" } else { "create" }.into(),
                    score: 100.0,
                    primary_action_id: if exists { "open" } else { "create" }.into(),
                    primary_action_label: if exists { "Open" } else { "Create" }.into(),
                    ..Default::default()
                }],
                vec![],
            )
            .await;
            return;
        }

        let rest_raw = query.rest_raw();
        let rest_check = rest_raw.trim().to_lowercase();

        if rest_check == "recent" {
            let index = self.index.clone();
            let handle = tokio::task::spawn_blocking(move || index.list_recent(20));
            let abort = handle.abort_handle();
            let hits = tokio::select! {
                _ = cancel.cancelled() => {
                    abort.abort();
                    return;
                }
                result = handle => match result {
                    Ok(Ok(hits)) => hits,
                    Ok(Err(error)) => {
                        self.emit_results(&sink, vec![unavailable_row("Notes recent unavailable", error.to_string())], vec![]).await;
                        return;
                    }
                    Err(error) => {
                        self.emit_results(&sink, vec![unavailable_row("Notes recent unavailable", error.to_string())], vec![]).await;
                        return;
                    }
                },
            };
            let status = self.index.scan_status();
            let mut upserts = Vec::new();
            if scan_status_needs_row(&status) {
                upserts.push(status_row(&status));
            }
            for hit in hits {
                if cancel.is_cancelled() {
                    return;
                }
                let path = match self
                    .workspace
                    .resolve_path(
                        root.clone(),
                        PathBuf::from(&hit.relative_path),
                        false,
                        cancel.clone(),
                    )
                    .await
                {
                    Ok(path) => path.path,
                    Err(NotesWorkspaceError::Cancelled) => return,
                    Err(_) => continue,
                };
                upserts.push(SearchItemDto {
                    id: format!("note:{}", path.display()),
                    module_id: "luma.notes".into(),
                    title: hit.title,
                    subtitle: Some(path.display().to_string()),
                    kind: "note".into(),
                    score: 80.0,
                    primary_action_id: "open".into(),
                    primary_action_label: "Open".into(),
                    ..Default::default()
                });
            }
            self.emit_results(&sink, upserts, vec![]).await;
            return;
        }

        if rest_check == "status" {
            let status = self.index.scan_status();
            let count_label = match self.index.document_count() {
                Ok(count) => format!("{count} documents indexed"),
                Err(error) => format!("document count unavailable: {error}"),
            };
            let count_kind = if count_label.starts_with("document count") {
                "unavailable"
            } else {
                "status"
            };
            self.emit_results(
                &sink,
                vec![
                    status_row(&status),
                    SearchItemDto {
                        id: "notes:doc-count".into(),
                        module_id: "luma.notes".into(),
                        title: count_label,
                        subtitle: Some(root.display().to_string()),
                        kind: count_kind.into(),
                        score: 90.0,
                        primary_action_id: "noop".into(),
                        primary_action_label: if count_kind == "unavailable" {
                            "Unavailable".into()
                        } else {
                            "Status".into()
                        },
                        ..Default::default()
                    },
                ],
                vec![],
            )
            .await;
            return;
        }

        if rest_check == "issues" {
            let index = self.index.clone();
            let handle = tokio::task::spawn_blocking(move || index.list_issues());
            let abort = handle.abort_handle();
            let issues = tokio::select! {
                _ = cancel.cancelled() => {
                    abort.abort();
                    return;
                }
                result = handle => match result {
                    Ok(Ok(issues)) => issues,
                    Ok(Err(error)) => {
                        self.emit_results(&sink, vec![unavailable_row("Notes issues unavailable", error.to_string())], vec![]).await;
                        return;
                    }
                    Err(error) => {
                        self.emit_results(&sink, vec![unavailable_row("Notes issues unavailable", error.to_string())], vec![]).await;
                        return;
                    }
                },
            };
            let mut upserts = Vec::new();
            for issue in issues.into_iter().take(query.limit) {
                if cancel.is_cancelled() {
                    return;
                }
                let path = match self
                    .workspace
                    .resolve_path(
                        root.clone(),
                        PathBuf::from(&issue.relative_path),
                        false,
                        cancel.clone(),
                    )
                    .await
                {
                    Ok(path) => path.path,
                    Err(NotesWorkspaceError::Cancelled) => return,
                    Err(_) => continue,
                };
                upserts.push(SearchItemDto {
                    id: format!("note:{}", path.display()),
                    module_id: "luma.notes".into(),
                    title: format!(
                        "{} — {}",
                        humanize_issue_type(&issue.issue_type),
                        issue.relative_path
                    ),
                    subtitle: Some(short_issue_message(&issue.message)),
                    kind: "issue".into(),
                    score: 50.0,
                    primary_action_id: "open".into(),
                    primary_action_label: "Open".into(),
                    ..Default::default()
                });
            }
            if upserts.is_empty() {
                upserts.push(SearchItemDto {
                    id: "notes:issues-empty".into(),
                    module_id: "luma.notes".into(),
                    title: "No scan issues".into(),
                    subtitle: Some("Index looks clean".into()),
                    kind: "status".into(),
                    score: 50.0,
                    primary_action_id: "noop".into(),
                    primary_action_label: "OK".into(),
                    ..Default::default()
                });
            }
            self.emit_results(&sink, upserts, vec![]).await;
            return;
        }

        if rest_check == "check" || rest_check == "reindex" {
            let index = self.index.clone();
            let root_clone = root.clone();
            let is_rebuild = rest_check == "reindex";
            let (flag, bridge) = Self::scan_cancel_bridge(&cancel);
            let report = tokio::task::spawn_blocking(move || {
                if is_rebuild {
                    index.rebuild(&root_clone, Some(flag))
                } else {
                    index.incremental_check(&root_clone, Some(flag))
                }
            })
            .await;
            if let Some(bridge) = bridge {
                bridge.abort();
            }
            let (title, kind, label) = match report {
                Ok(Ok(report)) if report.cancelled => {
                    ("Scan cancelled".into(), "unavailable", "Cancelled")
                }
                Ok(Ok(report)) => {
                    let title = if report.errors == 0 {
                        format!("Checked {} notes", report.processed)
                    } else {
                        format!(
                            "Checked {} notes · {} issue(s)",
                            report.processed, report.errors
                        )
                    };
                    (title, "status", "Done")
                }
                Ok(Err(error)) => (format!("Scan failed: {error}"), "unavailable", "Failed"),
                Err(error) => (format!("Scan failed: {error}"), "unavailable", "Failed"),
            };
            self.emit_results(
                &sink,
                vec![SearchItemDto {
                    id: "notes:scan-report".into(),
                    module_id: "luma.notes".into(),
                    title,
                    subtitle: Some(root.display().to_string()),
                    kind: kind.into(),
                    score: 100.0,
                    primary_action_id: "noop".into(),
                    primary_action_label: label.into(),
                    ..Default::default()
                }],
                vec![],
            )
            .await;
            return;
        }

        // Empty rest (`n `) and explicit browse both open the notes directory tree.
        if let Some(path_arg) = browse_argument(rest_raw, &rest_check) {
            let candidate = if path_arg.is_empty() {
                root.clone()
            } else {
                PathBuf::from(path_arg)
            };
            let (directory, listing) = match self
                .workspace
                .list_directory(root.clone(), candidate, BROWSE_MAX_ENTRIES, cancel.clone())
                .await
            {
                Ok(listing) => listing,
                Err(NotesWorkspaceError::Cancelled) => return,
                Err(NotesWorkspaceError::OutsideWorkspace) => {
                    self.emit_results(
                        &sink,
                        vec![SearchItemDto {
                            id: "notes:browse-denied".into(),
                            module_id: "luma.notes".into(),
                            title: "Path outside notes root".into(),
                            subtitle: Some(
                                "Notes paths must stay inside the configured root".into(),
                            ),
                            kind: "unavailable".into(),
                            score: 0.0,
                            primary_action_id: "noop".into(),
                            primary_action_label: "Unavailable".into(),
                            ..Default::default()
                        }],
                        vec![],
                    )
                    .await;
                    return;
                }
                Err(error) => {
                    self.emit_results(
                        &sink,
                        vec![unavailable_row("Cannot read folder", error.to_string())],
                        vec![],
                    )
                    .await;
                    return;
                }
            };
            let status = self.index.scan_status();
            let mut upserts = Vec::new();
            if scan_status_needs_row(&status) {
                upserts.push(status_row(&status));
            }
            if listing.truncated {
                upserts.push(SearchItemDto {
                    id: "notes:browse-truncated".into(),
                    module_id: "luma.notes".into(),
                    title: "Directory listing is limited".into(),
                    subtitle: Some(format!("Showing up to {BROWSE_MAX_ENTRIES} entries")),
                    kind: "status".into(),
                    score: 84.0,
                    primary_action_id: "noop".into(),
                    primary_action_label: "OK".into(),
                    ..Default::default()
                });
            }
            for entry in listing.entries {
                if cancel.is_cancelled() {
                    return;
                }
                let path = entry.path.path;
                match entry.kind {
                    NotesDirectoryEntryKind::Directory => upserts.push(SearchItemDto {
                        id: format!("browse:n:{}", path.display()),
                        module_id: "luma.notes".into(),
                        title: format!("{}/", entry.name),
                        subtitle: Some(path.display().to_string()),
                        kind: "directory".into(),
                        score: 85.0,
                        primary_action_id: "browse".into(),
                        primary_action_label: "Browse".into(),
                        ..Default::default()
                    }),
                    NotesDirectoryEntryKind::MarkdownFile => {
                        let title = entry.name[..entry.name.len().saturating_sub(3)].to_string();
                        upserts.push(SearchItemDto {
                            id: format!("note:{}", path.display()),
                            module_id: "luma.notes".into(),
                            title,
                            subtitle: Some(path.display().to_string()),
                            kind: "note".into(),
                            score: 75.0,
                            primary_action_id: "open".into(),
                            primary_action_label: "Open".into(),
                            ..Default::default()
                        });
                    }
                }
            }
            if upserts.is_empty() {
                upserts.push(SearchItemDto {
                    id: "notes:browse-empty".into(),
                    module_id: "luma.notes".into(),
                    title: "Empty folder".into(),
                    subtitle: Some(directory.path.display().to_string()),
                    kind: "status".into(),
                    score: 50.0,
                    primary_action_id: "noop".into(),
                    primary_action_label: "OK".into(),
                    ..Default::default()
                });
            }
            self.emit_results(&sink, upserts, vec![]).await;
            return;
        }

        let needle = rest_check;
        let index = self.index.clone();
        let limit = query.limit;
        let needle_for_search = needle.clone();
        let handle = tokio::task::spawn_blocking(move || index.search(&needle_for_search, limit));
        let abort = handle.abort_handle();
        let hits = tokio::select! {
            _ = cancel.cancelled() => {
                abort.abort();
                return;
            }
            result = handle => match result {
                Ok(Ok(hits)) => hits,
                Ok(Err(error)) => {
                    self.emit_results(&sink, vec![unavailable_row("Notes search unavailable", error.to_string())], vec![]).await;
                    return;
                }
                Err(error) => {
                    self.emit_results(&sink, vec![unavailable_row("Notes search unavailable", error.to_string())], vec![]).await;
                    return;
                }
            },
        };
        let mut upserts = Vec::new();
        for hit in hits {
            if cancel.is_cancelled() {
                return;
            }
            let path = match self
                .workspace
                .resolve_path(
                    root.clone(),
                    PathBuf::from(&hit.relative_path),
                    false,
                    cancel.clone(),
                )
                .await
            {
                Ok(path) => path.path,
                Err(NotesWorkspaceError::Cancelled) => return,
                Err(_) => continue,
            };
            upserts.push(SearchItemDto {
                id: format!("note:{}", path.display()),
                module_id: "luma.notes".into(),
                title: hit.title,
                subtitle: Some(if hit.snippet.is_empty() {
                    path.display().to_string()
                } else {
                    format!("{} — {}", path.display(), hit.snippet)
                }),
                kind: "note".into(),
                // bm25 is lower-is-better (often negative); invert for Luma descending score.
                score: 70.0 - hit.rank,
                primary_action_id: "open".into(),
                primary_action_label: "Open".into(),
                ..Default::default()
            });
        }
        if upserts.is_empty() {
            upserts.push(SearchItemDto {
                id: "notes:no-matches".into(),
                module_id: "luma.notes".into(),
                title: format!("No notes matching \"{needle}\""),
                subtitle: Some("Try another query · n browse · n recent".into()),
                kind: "status".into(),
                score: 50.0,
                primary_action_id: "noop".into(),
                primary_action_label: "OK".into(),
                ..Default::default()
            });
        }
        self.emit_results(&sink, upserts, vec![]).await;
    }
}
