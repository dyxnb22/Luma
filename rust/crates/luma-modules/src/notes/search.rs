use super::rows::{status_row, unavailable_row};
use super::NotesModule;
use luma_application::{NotesScanStatusView, SearchSink};
use luma_domain::Query;
use luma_protocol::SearchItemDto;
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

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

impl NotesModule {
    pub(super) async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        let root = self.root.read().await.clone();
        let Some(root) = root else {
            let row = SearchItemDto {
                id: "notes:configure".into(),
                module_id: "luma.notes".into(),
                title: "Choose a Notes root folder".into(),
                subtitle: Some("Run: luma config set --notes-root ~/Notes".into()),
                kind: "not_configured".into(),
                score: 0.0,
                primary_action_id: "seed_config".into(),
                primary_action_label: "Show command".into(),
                ..Default::default()
            };
            {
                let upserts = vec![row];
                self.emit_results(&sink, upserts, vec![]).await;
            }
            return;
        };

        let rest = query.rest_normalized();

        if rest == "new" {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let path = root.join("Inbox").join(format!("note-{stamp}.md"));
            {
                let upserts = vec![SearchItemDto {
                    id: format!("note:{}", path.display()),
                    module_id: "luma.notes".into(),
                    title: "Create note".into(),
                    subtitle: Some(path.display().to_string()),
                    kind: "create".into(),
                    score: 100.0,
                    primary_action_id: "create".into(),
                    primary_action_label: "Create".into(),
                    ..Default::default()
                }];
                self.emit_results(&sink, upserts, vec![]).await;
            }
            return;
        }

        if rest == "daily" || rest == "today" {
            let date = match self.clock.today_ymd() {
                Ok(d) => d,
                Err(err) => {
                    {
                        let upserts = vec![SearchItemDto {
                            id: "note:clock-error".into(),
                            module_id: "luma.notes".into(),
                            title: "Daily note unavailable".into(),
                            subtitle: Some(crate::ux::friendly_store_error(&err.to_string())),
                            kind: "unavailable".into(),
                            score: 0.0,
                            primary_action_id: "noop".into(),
                            primary_action_label: "Unavailable".into(),
                            ..Default::default()
                        }];
                        self.emit_results(&sink, upserts, vec![]).await;
                    }
                    return;
                }
            };
            let path = root.join("Daily").join(format!("{date}.md"));
            let exists = path.exists();
            {
                let upserts = vec![SearchItemDto {
                    id: format!("note:{}", path.display()),
                    module_id: "luma.notes".into(),
                    title: if exists {
                        format!("Open daily note ({date})")
                    } else {
                        format!("Create daily note ({date})")
                    },
                    subtitle: Some(path.display().to_string()),
                    kind: if exists {
                        "note".into()
                    } else {
                        "create".into()
                    },
                    score: 100.0,
                    primary_action_id: if exists {
                        "open".into()
                    } else {
                        "create".into()
                    },
                    primary_action_label: if exists {
                        "Open".into()
                    } else {
                        "Create".into()
                    },
                    ..Default::default()
                }];
                self.emit_results(&sink, upserts, vec![]).await;
            }
            return;
        }

        let rest_raw = query.rest_raw();
        let rest_check = rest_raw.trim().to_lowercase();

        if rest_check == "recent" {
            let index = self.index.clone();
            let limit = 20;
            let handle = tokio::task::spawn_blocking(move || index.list_recent(limit));
            let abort = handle.abort_handle();
            let hits = tokio::select! {
                            _ = cancel.cancelled() => {
                                abort.abort();
                                return;
                            }
                            result = handle => match result {
                                Ok(Ok(hits)) => hits,
                                Ok(Err(e)) => {
                                    {
                let upserts = vec![unavailable_row(
                                                "Notes recent unavailable",
                                                e.to_string(),
                                            )];
                self.emit_results(&sink, upserts, vec![]).await;
            }
                                    return;
                                }
                                Err(e) => {
                                    {
                let upserts = vec![unavailable_row(
                                                "Notes recent unavailable",
                                                e.to_string(),
                                            )];
                self.emit_results(&sink, upserts, vec![]).await;
            }
                                    return;
                                }
                            },
                        };
            let status = self.index.scan_status();
            let mut upserts = Vec::new();
            let show_status = match &status {
                NotesScanStatusView::Running { .. } | NotesScanStatusView::Failed { .. } => true,
                NotesScanStatusView::Completed { errors, .. } => *errors > 0,
                NotesScanStatusView::Idle => false,
            };
            if show_status {
                upserts.push(status_row(&status));
            }
            for hit in hits {
                let abs = root.join(&hit.relative_path);
                if NotesModule::resolve_under_root(&root, &abs).is_err() {
                    continue;
                }
                upserts.push(SearchItemDto {
                    id: format!("note:{}", abs.display()),
                    module_id: "luma.notes".into(),
                    title: hit.title,
                    subtitle: Some(abs.display().to_string()),
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
                Ok(n) => format!("{n} documents indexed"),
                Err(e) => format!("document count unavailable: {e}"),
            };
            let count_kind = if count_label.starts_with("document count") {
                "unavailable"
            } else {
                "status"
            };
            {
                let upserts = vec![
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
                ];
                self.emit_results(&sink, upserts, vec![]).await;
            }
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
                                Ok(Err(e)) => {
                                    {
                let upserts = vec![unavailable_row(
                                                "Notes issues unavailable",
                                                e.to_string(),
                                            )];
                self.emit_results(&sink, upserts, vec![]).await;
            }
                                    return;
                                }
                                Err(e) => {
                                    {
                let upserts = vec![unavailable_row(
                                                "Notes issues unavailable",
                                                e.to_string(),
                                            )];
                self.emit_results(&sink, upserts, vec![]).await;
            }
                                    return;
                                }
                            },
                        };
            let upserts: Vec<_> = issues
                .into_iter()
                .take(query.limit)
                .filter_map(|i| {
                    let abs = root.join(&i.relative_path);
                    if NotesModule::resolve_under_root(&root, &abs).is_err() {
                        return None;
                    }
                    Some(SearchItemDto {
                        id: format!("note:{}", abs.display()),
                        module_id: "luma.notes".into(),
                        title: format!(
                            "{} — {}",
                            humanize_issue_type(&i.issue_type),
                            i.relative_path
                        ),
                        subtitle: Some(short_issue_message(&i.message)),
                        kind: "issue".into(),
                        score: 50.0,
                        primary_action_id: "open".into(),
                        primary_action_label: "Open".into(),
                        ..Default::default()
                    })
                })
                .collect();
            {
                let upserts = if upserts.is_empty() {
                    vec![SearchItemDto {
                        id: "notes:issues-empty".into(),
                        module_id: "luma.notes".into(),
                        title: "No scan issues".into(),
                        subtitle: Some("Index looks clean".into()),
                        kind: "status".into(),
                        score: 50.0,
                        primary_action_id: "noop".into(),
                        primary_action_label: "OK".into(),
                        ..Default::default()
                    }]
                } else {
                    upserts
                };
                self.emit_results(&sink, upserts, vec![]).await;
            }
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
                Ok(Ok(r)) if r.cancelled => ("Scan cancelled".into(), "unavailable", "Cancelled"),
                Ok(Ok(r)) => {
                    let title = if r.errors == 0 {
                        format!("Checked {} notes", r.processed)
                    } else {
                        format!("Checked {} notes · {} issue(s)", r.processed, r.errors)
                    };
                    (title, "status", "Done")
                }
                Ok(Err(e)) => (format!("Scan failed: {e}"), "unavailable", "Failed"),
                Err(e) => (format!("Scan failed: {e}"), "unavailable", "Failed"),
            };
            {
                let upserts = vec![SearchItemDto {
                    id: "notes:scan-report".into(),
                    module_id: "luma.notes".into(),
                    title,
                    subtitle: Some(root.display().to_string()),
                    kind: kind.into(),
                    score: 100.0,
                    primary_action_id: "noop".into(),
                    primary_action_label: label.into(),
                    ..Default::default()
                }];
                self.emit_results(&sink, upserts, vec![]).await;
            }
            return;
        }

        // Empty rest (`n `) and explicit browse both open the notes directory tree.
        if rest_check.is_empty()
            || rest_check == "browse"
            || rest_check.starts_with("browse ")
            || rest_check.starts_with("ls ")
        {
            let path_arg = if rest_check.is_empty() {
                ""
            } else {
                rest_raw
                    .trim()
                    .strip_prefix("browse")
                    .or_else(|| rest_raw.trim().strip_prefix("Browse"))
                    .or_else(|| rest_raw.trim().strip_prefix("ls"))
                    .or_else(|| rest_raw.trim().strip_prefix("LS"))
                    .unwrap_or("")
                    .trim()
            };
            let dir = if path_arg.is_empty() {
                root.clone()
            } else {
                let candidate = PathBuf::from(path_arg);
                match NotesModule::resolve_under_root_for_browse(&root, &candidate) {
                    Ok(p) => p,
                    Err(err) => {
                        {
                            let upserts = vec![SearchItemDto {
                                id: "notes:browse-denied".into(),
                                module_id: "luma.notes".into(),
                                title: "Path outside notes root".into(),
                                subtitle: Some(err),
                                kind: "unavailable".into(),
                                score: 0.0,
                                primary_action_id: "noop".into(),
                                primary_action_label: "Unavailable".into(),
                                ..Default::default()
                            }];
                            self.emit_results(&sink, upserts, vec![]).await;
                        }
                        return;
                    }
                }
            };
            let Ok(rd) = std::fs::read_dir(&dir) else {
                {
                    let upserts = vec![unavailable_row(
                        "Cannot read folder",
                        dir.display().to_string(),
                    )];
                    self.emit_results(&sink, upserts, vec![]).await;
                }
                return;
            };
            let mut upserts = Vec::new();
            // Surface index problems on the default Notes surface (browse / `n `).
            let status = self.index.scan_status();
            let show_status = match &status {
                NotesScanStatusView::Running { .. } | NotesScanStatusView::Failed { .. } => true,
                NotesScanStatusView::Completed { errors, .. } => *errors > 0,
                NotesScanStatusView::Idle => false,
            };
            if show_status {
                upserts.push(status_row(&status));
            }
            let mut entries: Vec<_> = rd.flatten().collect();
            entries.sort_by_key(|e| e.file_name());
            for entry in entries {
                if cancel.is_cancelled() {
                    return;
                }
                let path = entry.path();
                let Ok(meta) = std::fs::symlink_metadata(&path) else {
                    continue;
                };
                // Never follow or enumerate symlinks during browse.
                if meta.file_type().is_symlink() {
                    continue;
                }
                let name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("?")
                    .to_string();
                if name.starts_with('.') {
                    continue;
                }
                if meta.file_type().is_dir() {
                    upserts.push(SearchItemDto {
                        id: format!("browse:n:{}", path.display()),
                        module_id: "luma.notes".into(),
                        title: format!("{name}/"),
                        subtitle: Some(path.display().to_string()),
                        kind: "directory".into(),
                        score: 85.0,
                        primary_action_id: "browse".into(),
                        primary_action_label: "Browse".into(),
                        ..Default::default()
                    });
                } else if meta.file_type().is_file() && name.to_ascii_lowercase().ends_with(".md") {
                    let title = if let Some(stem) = name
                        .get(..name.len().saturating_sub(3))
                        .filter(|_| name.len() >= 3)
                    {
                        // Preserve stem casing; strip any .md/.MD/.Md suffix length 3.
                        stem.to_string()
                    } else {
                        name.clone()
                    };
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
            if upserts.is_empty() {
                upserts.push(SearchItemDto {
                    id: "notes:browse-empty".into(),
                    module_id: "luma.notes".into(),
                    title: "Empty folder".into(),
                    subtitle: Some(dir.display().to_string()),
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
                        Ok(Err(e)) => {
                            {
            let upserts = vec![unavailable_row("Notes search unavailable", e.to_string())];
            self.emit_results(&sink, upserts, vec![]).await;
        }
                            return;
                        }
                        Err(e) => {
                            {
            let upserts = vec![unavailable_row("Notes search unavailable", e.to_string())];
            self.emit_results(&sink, upserts, vec![]).await;
        }
                            return;
                        }
                    },
                };
        let mut upserts = Vec::new();
        for hit in hits {
            if cancel.is_cancelled() {
                return;
            }
            let abs = root.join(&hit.relative_path);
            if NotesModule::resolve_under_root(&root, &abs).is_err() {
                continue;
            }
            upserts.push(SearchItemDto {
                id: format!("note:{}", abs.display()),
                module_id: "luma.notes".into(),
                title: hit.title,
                subtitle: Some(if hit.snippet.is_empty() {
                    abs.display().to_string()
                } else {
                    format!("{} — {}", abs.display(), hit.snippet)
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
