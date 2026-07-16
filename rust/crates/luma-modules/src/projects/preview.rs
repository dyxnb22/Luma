use super::Project;
use luma_application::{ImportedProject, ProjectDirectoryListing};
use std::path::PathBuf;

pub(super) fn imported_index(imported: &[ImportedProject], statuses: &[bool]) -> Vec<Project> {
    imported
        .iter()
        .enumerate()
        .map(|(index, p)| {
            let path = PathBuf::from(&p.path);
            let missing = !statuses.get(index).copied().unwrap_or(false);
            let name = p.name.clone().unwrap_or_else(|| {
                path.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("project")
                    .to_string()
            });
            Project {
                name,
                path,
                missing,
            }
        })
        .collect()
}

pub(super) fn format_projects_directory_preview(listing: ProjectDirectoryListing) -> String {
    const MAX: usize = 40;
    if listing.entries.is_empty() && !listing.truncated {
        return "Empty folder".into();
    }
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    for entry in listing.entries {
        if entry.is_directory {
            dirs.push(entry.name);
        } else {
            files.push(entry.name);
        }
    }
    let total = dirs.len() + files.len();
    let mut out = if listing.truncated {
        format!("At least {total} item(s):\n")
    } else {
        format!("{total} item(s):\n")
    };
    let mut shown = 0usize;
    for d in &dirs {
        if shown >= MAX {
            break;
        }
        out.push_str(&format!("  {d}/\n"));
        shown += 1;
    }
    for f in &files {
        if shown >= MAX {
            break;
        }
        out.push_str(&format!("  {f}\n"));
        shown += 1;
    }
    if shown < total {
        out.push_str(&format!("  … +{} more\n", total - shown));
    } else if listing.truncated {
        out.push_str("  … more not shown\n");
    }
    out.trim_end().to_string()
}
