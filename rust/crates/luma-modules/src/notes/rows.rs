use luma_application::NotesScanStatusView;
use luma_protocol::SearchItemDto;

pub(super) fn watch_warning_row(message: &str) -> SearchItemDto {
    SearchItemDto {
        id: "notes:watch-warning".into(),
        module_id: "luma.notes".into(),
        title: "Notes watcher stopped".into(),
        subtitle: Some(format!("{message} · Enter restarts")),
        kind: "unavailable".into(),
        score: 92.0,
        primary_action_id: "restart_watch".into(),
        primary_action_label: "Restart watcher".into(),
        ..Default::default()
    }
}

pub(super) fn unavailable_row(
    title: impl Into<String>,
    detail: impl Into<String>,
) -> SearchItemDto {
    let detail = crate::ux::friendly_store_error(&detail.into());
    SearchItemDto {
        id: "notes:unavailable".into(),
        module_id: "luma.notes".into(),
        title: title.into(),
        subtitle: Some(detail),
        kind: "unavailable".into(),
        score: 0.0,
        primary_action_id: "noop".into(),
        primary_action_label: "Unavailable".into(),
        ..Default::default()
    }
}

pub(super) fn status_row(status: &NotesScanStatusView) -> SearchItemDto {
    let (title, subtitle, score, action_id, action_label) = match status {
        NotesScanStatusView::Idle => ("Index idle".into(), "Ready".into(), 10.0, "noop", "Status"),
        NotesScanStatusView::Running {
            processed, total, ..
        } => (
            "Updating notes index…".into(),
            format!("{processed}/{total}"),
            95.0,
            "noop",
            "Status",
        ),
        NotesScanStatusView::Failed { message } => (
            "Notes index failed".into(),
            crate::ux::friendly_store_error(message),
            95.0,
            "noop",
            "Status",
        ),
        NotesScanStatusView::Completed {
            processed,
            total,
            errors,
            ..
        } => {
            if *errors > 0 {
                (
                    format!("{errors} index issue(s)"),
                    format!("{processed}/{total} notes · Enter for details"),
                    90.0,
                    "list_issues",
                    "View issues",
                )
            } else {
                (
                    "Notes index OK".into(),
                    format!("{processed}/{total}"),
                    10.0,
                    "noop",
                    "Status",
                )
            }
        }
    };
    SearchItemDto {
        id: "notes:index-status".into(),
        module_id: "luma.notes".into(),
        title,
        subtitle: Some(subtitle),
        kind: "status".into(),
        score,
        primary_action_id: action_id.into(),
        primary_action_label: action_label.into(),
        ..Default::default()
    }
}
