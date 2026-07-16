use super::NotesModule;
use luma_application::{NotesDirectoryEntryKind, NotesDirectoryListing, NotesWorkspacePreview};
use luma_domain::SearchItem;
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

const PREVIEW_MAX_BYTES: usize = 4_096;
const PREVIEW_MAX_ENTRIES: usize = 64;

fn result_candidate(result: &SearchItem) -> Option<PathBuf> {
    result
        .id
        .as_str()
        .strip_prefix("browse:n:")
        .or_else(|| result.id.as_str().strip_prefix("note:"))
        .map(PathBuf::from)
}

fn format_notes_directory_preview(listing: &NotesDirectoryListing) -> String {
    let mut directories = Vec::new();
    let mut files = Vec::new();
    for entry in &listing.entries {
        match entry.kind {
            NotesDirectoryEntryKind::Directory => directories.push(entry.name.as_str()),
            NotesDirectoryEntryKind::MarkdownFile => files.push(entry.name.as_str()),
        }
    }
    let total = directories.len() + files.len();
    if total == 0 && !listing.truncated {
        return "Empty folder".into();
    }
    let mut out = if listing.truncated {
        format!("At least {total} item(s):\n")
    } else {
        format!("{total} item(s):\n")
    };
    for directory in directories {
        out.push_str(&format!("  {directory}/\n"));
    }
    for file in files {
        out.push_str(&format!("  {file}\n"));
    }
    if listing.truncated {
        out.push_str("  … more entries not shown\n");
    }
    out.trim_end().to_string()
}

impl NotesModule {
    pub(super) async fn preview(&self, result: &SearchItem) -> Option<String> {
        let root = self.root.read().await.clone()?;
        let candidate = result_candidate(result)?;
        let (path, preview) = self
            .workspace
            .preview(
                root,
                candidate,
                PREVIEW_MAX_BYTES,
                PREVIEW_MAX_ENTRIES,
                CancellationToken::new(),
            )
            .await
            .ok()?;

        match preview {
            NotesWorkspacePreview::Directory(listing) => {
                Some(format_notes_directory_preview(&listing))
            }
            NotesWorkspacePreview::File(body) => {
                let mut out = String::new();
                if let Ok(Some(doc)) = self.index.get_document(&path.relative_path) {
                    out.push_str(&format!("# {}\n", doc.title));
                    out.push_str(&format!("path: {}\n", doc.relative_path));
                    out.push_str(&format!("mtime: {}\n", doc.mtime_unix));
                    out.push_str(&format!("size: {}\n", doc.size_bytes));
                    if !doc.tags.is_empty() {
                        out.push_str(&format!("tags: {}\n", doc.tags.join(", ")));
                    }
                    if !doc.outbound.is_empty() {
                        out.push_str("outbound:\n");
                        for link in doc.outbound.iter().take(12) {
                            out.push_str(&format!("  - {} ({})\n", link.raw_href, link.kind));
                        }
                    }
                    if !doc.backlinks.is_empty() {
                        out.push_str("backlinks:\n");
                        for link in doc.backlinks.iter().take(12) {
                            out.push_str(&format!("  - {}\n", link.target_path));
                        }
                    }
                    out.push('\n');
                }
                out.push_str(&body);
                Some(out)
            }
            NotesWorkspacePreview::Unsupported => result.subtitle.clone(),
        }
    }
}
