//! Resume: explicit work-entry contexts (list / save / edit / restore).
//!
//! No background tracking — only user-triggered create/update/resume/delete.

use crate::cancel::await_unless_cancelled;
use async_trait::async_trait;
use luma_application::{
    blank_context, normalize_resume_name, normalize_resume_path, resume_now_iso, ssh_connect_args,
    ActionOutcome, ActionRequest, GitInfoError, GitInfoPort, LumaModule, ModuleManifest,
    ModuleState, OpenEditorPort, OpenPathPort, ResumeContext, ResumeContextsRepository,
    ResumeEditor, ResumeRecipeRef, SearchMode, SearchSink, WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, RecipeRisk,
    RecipeRunPlan, ResolvedCommandStep, SearchItem,
};
use luma_protocol::{Event, SearchItemDto, UiIntent};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

const MODULE_ID: &str = "luma.resume";

pub struct ResumeModule {
    manifest: ModuleManifest,
    store: Arc<dyn ResumeContextsRepository>,
    opener: Arc<dyn OpenPathPort>,
    editor: Arc<dyn OpenEditorPort>,
    git: Arc<dyn GitInfoPort>,
    store_error: RwLock<Option<String>>,
}

impl ResumeModule {
    pub fn with_deps(
        store: Arc<dyn ResumeContextsRepository>,
        opener: Arc<dyn OpenPathPort>,
        editor: Arc<dyn OpenEditorPort>,
        git: Arc<dyn GitInfoPort>,
    ) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new(MODULE_ID),
                display_name: "Resume".into(),
                triggers: vec!["resume".into(), "rs".into()],
                default_enabled: true,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
                workbench: luma_application::WorkbenchMeta {
                    glyph: Some("R".into()),
                    suggested_query: Some("resume ".into()),
                    empty_hint: Some(
                        "resume · resume save <name> · resume capture <name>".into(),
                    ),
                    supports_browse: false,
                },
            },
            store,
            opener,
            editor,
            git,
            store_error: RwLock::new(None),
        }
    }

    async fn set_store_error(&self, err: Option<String>) {
        *self.store_error.write().await = err;
    }

    fn context_id(name: &str) -> String {
        format!("resume:{name}")
    }

    fn context_name_from_id(id: &str) -> Option<&str> {
        id.strip_prefix("resume:")
            .filter(|n| !n.is_empty() && !n.contains(':'))
    }

    fn subtitle_for(ctx: &ResumeContext) -> String {
        let mut parts = Vec::new();
        if let Some(p) = ctx.project_path.as_deref() {
            parts.push(short_path(p));
        }
        if let Some(b) = ctx.git_branch.as_deref() {
            parts.push(b.to_string());
        }
        if let Some(w) = ctx.worktree_path.as_deref() {
            if ctx.project_path.as_deref() != Some(w) {
                parts.push(format!("wt:{}", short_path(w)));
            }
        }
        if let Some(t) = ctx.last_resumed_at.as_deref() {
            parts.push(format!("resumed {t}"));
        } else {
            parts.push(format!("updated {}", ctx.updated_at));
        }
        parts.join(" · ")
    }

    fn preview_text(ctx: &ResumeContext) -> String {
        let mut lines = vec![
            format!("name: {}", ctx.name),
            format!("display: {}", ctx.display_name),
            format!(
                "project: {}",
                ctx.project_path.as_deref().unwrap_or("(none)")
            ),
            format!("branch: {}", ctx.git_branch.as_deref().unwrap_or("(none)")),
            format!(
                "worktree: {}",
                ctx.worktree_path.as_deref().unwrap_or("(none)")
            ),
            format!("ssh: {}", ctx.ssh_host.as_deref().unwrap_or("(none)")),
            format!(
                "editor: {}",
                ctx.editor
                    .as_ref()
                    .map(ResumeEditor::as_str)
                    .unwrap_or("(none)")
            ),
            format!(
                "editor_project: {}",
                ctx.editor_project_path.as_deref().unwrap_or("(none)")
            ),
            format!(
                "terminal_cwd: {}",
                ctx.terminal_cwd.as_deref().unwrap_or("(none)")
            ),
            format!("created: {}", ctx.created_at),
            format!("updated: {}", ctx.updated_at),
            format!(
                "last_resumed: {}",
                ctx.last_resumed_at.as_deref().unwrap_or("(never)")
            ),
        ];
        if ctx.documents.is_empty() {
            lines.push("documents: (none)".into());
        } else {
            lines.push("documents:".into());
            for d in &ctx.documents {
                lines.push(format!("  - {d}"));
            }
        }
        if ctx.notes.is_empty() {
            lines.push("notes: (none)".into());
        } else {
            lines.push("notes:".into());
            for n in &ctx.notes {
                lines.push(format!("  - {n}"));
            }
        }
        if ctx.recipes.is_empty() {
            lines.push("recipes: (none) — not run on resume".into());
        } else {
            lines.push("recipes (display only; confirm to run):".into());
            for r in &ctx.recipes {
                match &r.command {
                    Some(cmd) => lines.push(format!("  - {}: {cmd}", r.name)),
                    None => lines.push(format!("  - {}", r.name)),
                }
            }
        }
        lines.push(String::new());
        lines.push("Safe restore opens project/docs/editor/notes.".into());
        lines.push("Connect SSH and Run recipe always require confirmation.".into());
        lines.join("\n")
    }

    fn dto_for_context(ctx: &ResumeContext, score: f64) -> SearchItemDto {
        SearchItemDto {
            id: Self::context_id(&ctx.name),
            module_id: MODULE_ID.into(),
            title: if ctx.display_name != ctx.name {
                format!("{} ({})", ctx.display_name, ctx.name)
            } else {
                ctx.name.clone()
            },
            subtitle: Some(Self::subtitle_for(ctx)),
            kind: "resume".into(),
            score,
            primary_action_id: "resume_entry".into(),
            primary_action_label: "Resume entry".into(),
            ui_intent: None,
            action_payload: Some(serde_json::json!({ "name": ctx.name })),
            ..Default::default()
        }
    }

    fn empty_row() -> SearchItemDto {
        SearchItemDto {
            id: "resume:empty".into(),
            module_id: MODULE_ID.into(),
            title: "No resume contexts yet".into(),
            subtitle: Some("Create one: resume save <name> · or resume capture <name>".into()),
            kind: "not_configured".into(),
            score: 1.0,
            primary_action_id: "noop".into(),
            primary_action_label: "OK".into(),
            ui_intent: Some(UiIntent::SeedConfig),
            action_payload: None,
            ..Default::default()
        }
    }

    fn unavailable_row(reason: String) -> SearchItemDto {
        SearchItemDto {
            id: "resume:unavailable".into(),
            module_id: MODULE_ID.into(),
            title: "Resume store unavailable".into(),
            subtitle: Some(crate::ux::friendly_store_error(&reason)),
            kind: "unavailable".into(),
            score: 1.0,
            primary_action_id: "rebuild_store".into(),
            primary_action_label: "Rebuild empty store".into(),
            primary_action_risk: ActionRisk::Confirm,
            primary_action_confirmation: true,
            ui_intent: None,
            action_payload: None,
            ..Default::default()
        }
    }

    fn not_found_row(name: &str) -> SearchItemDto {
        SearchItemDto {
            id: format!("resume:missing:{name}"),
            module_id: MODULE_ID.into(),
            title: format!("No resume context named “{name}”"),
            subtitle: Some(format!(
                "Try: resume save {name} · resume capture {name} · resume list"
            )),
            kind: "not_configured".into(),
            score: 1.0,
            primary_action_id: "noop".into(),
            primary_action_label: "OK".into(),
            ui_intent: None,
            action_payload: None,
            ..Default::default()
        }
    }

    fn save_preview_row(name: &str, exists: bool) -> SearchItemDto {
        let (title, kind, label) = if exists {
            (
                format!("Update resume context “{name}”"),
                "update",
                "Save / update",
            )
        } else {
            (
                format!("Create resume context “{name}”"),
                "create",
                "Save",
            )
        };
        SearchItemDto {
            id: format!("resume:save:{name}"),
            module_id: MODULE_ID.into(),
            title: title.into(),
            subtitle: Some(
                "Enter to upsert · use resume capture <name> to fill from cwd/git".into(),
            ),
            kind: kind.into(),
            score: 10.0,
            primary_action_id: "save".into(),
            primary_action_label: label.into(),
            ui_intent: None,
            action_payload: Some(serde_json::json!({ "name": name })),
            ..Default::default()
        }
    }

    fn capture_preview_row(name: &str) -> SearchItemDto {
        SearchItemDto {
            id: format!("resume:capture:{name}"),
            module_id: MODULE_ID.into(),
            title: format!("Capture cwd into “{name}”"),
            subtitle: Some(
                "Reads project/branch/worktree from current directory, then saves".into(),
            ),
            kind: "create".into(),
            score: 10.0,
            primary_action_id: "capture".into(),
            primary_action_label: "Capture & save".into(),
            ui_intent: None,
            action_payload: Some(serde_json::json!({ "name": name })),
            ..Default::default()
        }
    }

    fn delete_preview_row(name: &str) -> SearchItemDto {
        SearchItemDto {
            id: format!("resume:delete:{name}"),
            module_id: MODULE_ID.into(),
            title: format!("Delete resume context “{name}”"),
            subtitle: Some("Requires confirmation".into()),
            kind: "delete".into(),
            score: 10.0,
            primary_action_id: "delete".into(),
            primary_action_label: "Delete".into(),
            primary_action_risk: ActionRisk::Destructive,
            primary_action_confirmation: true,
            ui_intent: None,
            action_payload: Some(serde_json::json!({ "name": name })),
            ..Default::default()
        }
    }

    fn set_preview_row(name: &str, field: &str, value: &str) -> SearchItemDto {
        SearchItemDto {
            id: format!("resume:set:{name}:{field}"),
            module_id: MODULE_ID.into(),
            title: format!("Set {field} on “{name}”"),
            subtitle: Some(value.to_string()),
            kind: "update".into(),
            score: 10.0,
            primary_action_id: "set_field".into(),
            primary_action_label: "Save field".into(),
            ui_intent: None,
            action_payload: Some(serde_json::json!({
                "name": name,
                "field": field,
                "value": value,
            })),
            ..Default::default()
        }
    }

    async fn emit(&self, sink: &SearchSink, upserts: Vec<SearchItemDto>) {
        let _ = sink
            .send(Event::ResultsChunk {
                request_id: String::new(),
                sequence: 1,
                upserts,
                removed_ids: vec![],
            })
            .await;
    }

    async fn load_or_unavailable(&self, sink: &SearchSink) -> Option<Vec<ResumeContext>> {
        match self.store.list() {
            Ok(list) => {
                self.set_store_error(None).await;
                Some(list)
            }
            Err(err) => {
                let msg = err.to_string();
                self.set_store_error(Some(msg.clone())).await;
                self.emit(sink, vec![Self::unavailable_row(msg)]).await;
                None
            }
        }
    }

    fn resolve_name_from_item(item: &SearchItem) -> Option<String> {
        if let Some(payload) = &item.action_payload {
            if let Some(name) = payload.get("name").and_then(|v| v.as_str()) {
                return normalize_resume_name(name).ok();
            }
        }
        if let Some(name) = Self::context_name_from_id(item.id.as_str()) {
            return Some(name.to_string());
        }
        for prefix in ["resume:save:", "resume:capture:", "resume:delete:", "resume:missing:"] {
            if let Some(name) = item.id.as_str().strip_prefix(prefix) {
                return normalize_resume_name(name).ok();
            }
        }
        if let Some(rest) = item.id.as_str().strip_prefix("resume:set:") {
            let name = rest.split(':').next().unwrap_or("");
            return normalize_resume_name(name).ok();
        }
        None
    }

    async fn open_path_step(
        &self,
        label: &str,
        path: Option<&str>,
        cancel: &CancellationToken,
        reports: &mut Vec<String>,
    ) -> bool {
        let Some(raw) = path.filter(|p| !p.is_empty()) else {
            reports.push(format!("{label}: skipped (not set)"));
            return false;
        };
        let p = PathBuf::from(raw);
        if !p.exists() {
            reports.push(format!("{label}: unavailable (missing {raw})"));
            return false;
        }
        match await_unless_cancelled(cancel, self.opener.open(&p)).await {
            None => {
                reports.push(format!("{label}: cancelled"));
                false
            }
            Some(Ok(())) => {
                reports.push(format!("{label}: ok"));
                true
            }
            Some(Err(err)) => {
                reports.push(format!("{label}: failed ({err})"));
                false
            }
        }
    }

    async fn resume_entry(&self, name: &str, cancel: CancellationToken) -> ActionOutcome {
        let ctx = match self.store.get(name) {
            Ok(Some(c)) => c,
            Ok(None) => {
                return ActionOutcome::Failed {
                    kind: FailureKind::NotFound {
                        entity: format!("resume:{name}"),
                    },
                };
            }
            Err(err) => {
                return ActionOutcome::Failed {
                    kind: FailureKind::Unavailable {
                        reason: err.to_string(),
                        retryable: true,
                    },
                };
            }
        };

        let mut reports = Vec::new();
        let mut any_ok = false;

        if self
            .open_path_step("project", ctx.project_path.as_deref(), &cancel, &mut reports)
            .await
        {
            any_ok = true;
        }
        if self
            .open_path_step(
                "worktree",
                ctx.worktree_path.as_deref().filter(|w| {
                    ctx.project_path.as_deref() != Some(*w)
                }),
                &cancel,
                &mut reports,
            )
            .await
        {
            any_ok = true;
        }
        for doc in &ctx.documents {
            if self
                .open_path_step("doc", Some(doc.as_str()), &cancel, &mut reports)
                .await
            {
                any_ok = true;
            }
        }
        for note in &ctx.notes {
            if self
                .open_path_step("note", Some(note.as_str()), &cancel, &mut reports)
                .await
            {
                any_ok = true;
            }
        }

        if let (Some(editor), Some(path)) = (ctx.editor.clone(), ctx.editor_project_path.clone()) {
            let p = PathBuf::from(&path);
            if !p.exists() {
                reports.push(format!("editor: unavailable (missing {path})"));
            } else {
                match await_unless_cancelled(&cancel, self.editor.open(editor, &p)).await {
                    None => reports.push("editor: cancelled".into()),
                    Some(Ok(())) => {
                        reports.push("editor: ok".into());
                        any_ok = true;
                    }
                    Some(Err(err)) => reports.push(format!("editor: failed ({err})")),
                }
            }
        } else {
            reports.push("editor: skipped (not set)".into());
        }

        // Display-only — never auto-run.
        if let Some(host) = ctx.ssh_host.as_deref() {
            reports.push(format!("ssh: {host} (not connected — use Connect SSH)"));
        } else {
            reports.push("ssh: (none)".into());
        }
        if ctx.recipes.is_empty() {
            reports.push("recipes: (none)".into());
        } else {
            let names: Vec<_> = ctx.recipes.iter().map(|r| r.name.as_str()).collect();
            reports.push(format!(
                "recipes: {} (not run — use Run recipe)",
                names.join(", ")
            ));
        }
        if let Some(cwd) = ctx.terminal_cwd.as_deref() {
            reports.push(format!("terminal_cwd: {cwd} (not opened)"));
        }

        if any_ok || ctx.project_path.is_some() || ctx.editor_project_path.is_some() {
            if let Err(err) = self.store.mark_resumed(name) {
                reports.push(format!("last_resumed_at: failed to update ({err})"));
            } else {
                reports.push("last_resumed_at: updated".into());
            }
        }

        ActionOutcome::Success {
            message: Some(reports.join(" · ")),
        }
    }

    async fn perform_save(&self, name: &str) -> ActionOutcome {
        let key = match normalize_resume_name(name) {
            Ok(k) => k,
            Err(err) => {
                return ActionOutcome::Failed {
                    kind: FailureKind::InvalidInput {
                        field: "name".into(),
                        message: err.to_string(),
                    },
                };
            }
        };
        let mut ctx = match self.store.get(&key) {
            Ok(Some(existing)) => existing,
            Ok(None) => match blank_context(&key) {
                Ok(c) => c,
                Err(err) => {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "name".into(),
                            message: err.to_string(),
                        },
                    };
                }
            },
            Err(err) => {
                return ActionOutcome::Failed {
                    kind: FailureKind::Unavailable {
                        reason: err.to_string(),
                        retryable: true,
                    },
                };
            }
        };
        ctx.updated_at = resume_now_iso();
        match self.store.upsert(ctx) {
            Ok(saved) => ActionOutcome::Success {
                message: Some(format!(
                    "saved “{}” — set fields with resume set {} <field> <value>",
                    saved.name, saved.name
                )),
            },
            Err(err) => ActionOutcome::Failed {
                kind: FailureKind::Unavailable {
                    reason: err.to_string(),
                    retryable: true,
                },
            },
        }
    }

    async fn perform_capture(&self, name: &str, cancel: CancellationToken) -> ActionOutcome {
        let key = match normalize_resume_name(name) {
            Ok(k) => k,
            Err(err) => {
                return ActionOutcome::Failed {
                    kind: FailureKind::InvalidInput {
                        field: "name".into(),
                        message: err.to_string(),
                    },
                };
            }
        };
        let cwd = match std::env::current_dir() {
            Ok(c) => c,
            Err(err) => {
                return ActionOutcome::Failed {
                    kind: FailureKind::Unavailable {
                        reason: format!("current directory unavailable: {err}"),
                        retryable: true,
                    },
                };
            }
        };
        let mut ctx = match self.store.get(&key) {
            Ok(Some(existing)) => existing,
            Ok(None) => match blank_context(&key) {
                Ok(c) => c,
                Err(err) => {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "name".into(),
                            message: err.to_string(),
                        },
                    };
                }
            },
            Err(err) => {
                return ActionOutcome::Failed {
                    kind: FailureKind::Unavailable {
                        reason: err.to_string(),
                        retryable: true,
                    },
                };
            }
        };

        let mut notes = Vec::new();
        let cwd_str = cwd.display().to_string();
        ctx.terminal_cwd = Some(cwd_str.clone());

        match await_unless_cancelled(&cancel, self.git.inspect(&cwd)).await {
            None => return ActionOutcome::Cancelled,
            Some(Ok(snap)) => {
                ctx.project_path = Some(snap.repo_root.display().to_string());
                ctx.worktree_path = Some(snap.worktree_path.display().to_string());
                ctx.git_branch = snap.branch;
                notes.push("git: ok".into());
            }
            Some(Err(GitInfoError::NotARepo)) => {
                ctx.project_path = Some(cwd_str.clone());
                ctx.worktree_path = None;
                ctx.git_branch = None;
                notes.push("git: not a repository — saved cwd as project_path".into());
            }
            Some(Err(GitInfoError::Unavailable(reason))) => {
                ctx.project_path = Some(cwd_str.clone());
                notes.push(format!("git: unavailable ({reason}) — saved cwd as project_path"));
            }
        }

        if ctx.editor.is_none() {
            ctx.editor = Some(ResumeEditor::Cursor);
        }
        if ctx.editor_project_path.is_none() {
            ctx.editor_project_path = ctx.project_path.clone();
        }
        ctx.updated_at = resume_now_iso();

        match self.store.upsert(ctx) {
            Ok(saved) => ActionOutcome::Success {
                message: Some(format!(
                    "captured “{}” → {} · {}",
                    saved.name,
                    saved.project_path.as_deref().unwrap_or("?"),
                    notes.join(" · ")
                )),
            },
            Err(err) => ActionOutcome::Failed {
                kind: FailureKind::Unavailable {
                    reason: err.to_string(),
                    retryable: true,
                },
            },
        }
    }

    fn apply_field(
        ctx: &mut ResumeContext,
        field: &str,
        value: &str,
    ) -> Result<(), FailureKind> {
        let path_fields = [
            "project_path",
            "worktree_path",
            "editor_project_path",
            "terminal_cwd",
        ];
        match field {
            "display_name" => {
                ctx.display_name = value.trim().to_string();
            }
            "git_branch" | "branch" => {
                ctx.git_branch = empty_to_none(value);
            }
            "ssh_host" | "ssh" => {
                ctx.ssh_host = empty_to_none(value);
            }
            "editor" => {
                if value.trim().is_empty() {
                    ctx.editor = None;
                } else {
                    ctx.editor = Some(ResumeEditor::parse(value).ok_or_else(|| {
                        FailureKind::InvalidInput {
                            field: "editor".into(),
                            message: "use cursor|vscode|intellij|default".into(),
                        }
                    })?);
                }
            }
            "documents" | "docs" => {
                ctx.documents = split_paths(value)?;
            }
            "notes" => {
                ctx.notes = split_paths(value)?;
            }
            "recipes" => {
                ctx.recipes = parse_recipes(value)?;
            }
            f if path_fields.contains(&f) => {
                let normalized = if value.trim().is_empty() {
                    None
                } else {
                    Some(normalize_resume_path(value).map_err(|e| {
                        FailureKind::InvalidInput {
                            field: f.to_string(),
                            message: e.to_string(),
                        }
                    })?)
                };
                match f {
                    "project_path" => ctx.project_path = normalized,
                    "worktree_path" => ctx.worktree_path = normalized,
                    "editor_project_path" => ctx.editor_project_path = normalized,
                    "terminal_cwd" => ctx.terminal_cwd = normalized,
                    _ => unreachable!(),
                }
            }
            other => {
                return Err(FailureKind::InvalidInput {
                    field: other.into(),
                    message: "unknown field — project_path|git_branch|worktree_path|ssh_host|editor|editor_project_path|documents|notes|recipes|terminal_cwd|display_name".into(),
                });
            }
        }
        ctx.updated_at = resume_now_iso();
        Ok(())
    }

    async fn perform_set(&self, name: &str, field: &str, value: &str) -> ActionOutcome {
        let key = match normalize_resume_name(name) {
            Ok(k) => k,
            Err(err) => {
                return ActionOutcome::Failed {
                    kind: FailureKind::InvalidInput {
                        field: "name".into(),
                        message: err.to_string(),
                    },
                };
            }
        };
        let mut ctx = match self.store.get(&key) {
            Ok(Some(c)) => c,
            Ok(None) => match blank_context(&key) {
                Ok(c) => c,
                Err(err) => {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "name".into(),
                            message: err.to_string(),
                        },
                    };
                }
            },
            Err(err) => {
                return ActionOutcome::Failed {
                    kind: FailureKind::Unavailable {
                        reason: err.to_string(),
                        retryable: true,
                    },
                };
            }
        };
        if let Err(kind) = Self::apply_field(&mut ctx, field, value) {
            return ActionOutcome::Failed { kind };
        }
        match self.store.upsert(ctx) {
            Ok(saved) => ActionOutcome::Success {
                message: Some(format!("updated {field} on “{}”", saved.name)),
            },
            Err(err) => ActionOutcome::Failed {
                kind: FailureKind::Unavailable {
                    reason: err.to_string(),
                    retryable: true,
                },
            },
        }
    }

    fn recipe_plan(ctx: &ResumeContext, index: usize) -> Result<RecipeRunPlan, FailureKind> {
        let recipe = ctx.recipes.get(index).ok_or_else(|| FailureKind::NotFound {
            entity: format!("recipe:{index}"),
        })?;
        let command = recipe.command.as_deref().filter(|c| !c.is_empty()).ok_or_else(|| {
            FailureKind::NotConfigured {
                remediation: format!(
                    "recipe “{}” has no command — set with resume set {} recipes name=cmd",
                    recipe.name, ctx.name
                ),
            }
        })?;
        let cwd = ctx
            .terminal_cwd
            .as_ref()
            .or(ctx.project_path.as_ref())
            .map(PathBuf::from)
            .ok_or_else(|| FailureKind::NotConfigured {
                remediation: format!(
                    "set terminal_cwd or project_path before running recipes (resume set {} terminal_cwd <path>)",
                    ctx.name
                ),
            })?;
        if !cwd.is_dir() {
            return Err(FailureKind::Unavailable {
                reason: format!("cwd not found: {}", cwd.display()),
                retryable: false,
            });
        }
        let (program, args) = split_command(command);
        Ok(RecipeRunPlan {
            recipe_id: format!("resume:{}:{}", ctx.name, recipe.name),
            recipe_title: recipe.name.clone(),
            risk: RecipeRisk::Confirm,
            working_dir: cwd.clone(),
            variant_id: "saved".into(),
            variant_description: command.to_string(),
            steps: vec![ResolvedCommandStep {
                id: "run".into(),
                label: recipe.name.clone(),
                program,
                args,
                cwd: cwd.clone(),
                root: cwd,
                continue_on_error: false,
            }],
        })
    }
}

fn short_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| path.to_string())
}

fn empty_to_none(value: &str) -> Option<String> {
    let t = value.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

fn split_paths(value: &str) -> Result<Vec<String>, FailureKind> {
    let mut out = Vec::new();
    for part in value.split('|') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        out.push(
            normalize_resume_path(part).map_err(|e| FailureKind::InvalidInput {
                field: "path".into(),
                message: e.to_string(),
            })?,
        );
    }
    Ok(out)
}

fn parse_recipes(value: &str) -> Result<Vec<ResumeRecipeRef>, FailureKind> {
    let mut out = Vec::new();
    for part in value.split('|') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((name, cmd)) = part.split_once('=') {
            out.push(ResumeRecipeRef {
                name: name.trim().to_string(),
                command: Some(cmd.trim().to_string()),
            });
        } else {
            out.push(ResumeRecipeRef {
                name: part.to_string(),
                command: None,
            });
        }
    }
    Ok(out)
}

fn split_command(command: &str) -> (String, Vec<String>) {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for ch in command.chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            c if c.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    parts.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        parts.push(current);
    }
    if parts.is_empty() {
        return ("/bin/zsh".into(), vec!["-lc".into(), command.to_string()]);
    }
    let program = parts.remove(0);
    (program, parts)
}

fn action(id: &str, label: &str, risk: ActionRisk, confirmation: bool) -> ActionDescriptor {
    ActionDescriptor {
        id: ActionId::new(id),
        label: label.into(),
        risk,
        confirmation,
    }
}

#[async_trait]
impl LumaModule for ResumeModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
        match self.store.list() {
            Ok(_) => {
                self.set_store_error(None).await;
                ModuleState::Ready
            }
            Err(err) => {
                let msg = err.to_string();
                self.set_store_error(Some(msg.clone())).await;
                ModuleState::Failed(msg)
            }
        }
    }

    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        if cancel.is_cancelled() {
            return;
        }
        let rest = query.rest_normalized();
        let rest_raw = query.rest_raw();

        if rest.is_empty() || rest == "list" {
            let Some(list) = self.load_or_unavailable(&sink).await else {
                return;
            };
            if list.is_empty() {
                self.emit(&sink, vec![Self::empty_row()]).await;
                return;
            }
            let upserts = list
                .iter()
                .enumerate()
                .map(|(i, ctx)| Self::dto_for_context(ctx, 100.0 - i as f64))
                .collect();
            self.emit(&sink, upserts).await;
            return;
        }

        if let Some(body) = rest.strip_prefix("save ") {
            let name = body.trim();
            if name.is_empty() {
                self.emit(
                    &sink,
                    vec![SearchItemDto {
                        id: "resume:save-help".into(),
                        module_id: MODULE_ID.into(),
                        title: "Save a resume context".into(),
                        subtitle: Some("resume save <name>".into()),
                        kind: "not_configured".into(),
                        score: 1.0,
                        primary_action_id: "noop".into(),
                        primary_action_label: "OK".into(),
                        ui_intent: None,
                        action_payload: None,
                    ..Default::default()
                    }],
                )
                .await;
                return;
            }
            let key = match normalize_resume_name(name) {
                Ok(k) => k,
                Err(err) => {
                    self.emit(
                        &sink,
                        vec![SearchItemDto {
                            id: "resume:invalid-name".into(),
                            module_id: MODULE_ID.into(),
                            title: "Invalid name".into(),
                            subtitle: Some(err.to_string()),
                            kind: "unavailable".into(),
                            score: 1.0,
                            primary_action_id: "noop".into(),
                            primary_action_label: "OK".into(),
                            ui_intent: None,
                            action_payload: None,
                        ..Default::default()
                        }],
                    )
                    .await;
                    return;
                }
            };
            let exists = matches!(self.store.get(&key), Ok(Some(_)));
            self.emit(&sink, vec![Self::save_preview_row(&key, exists)])
                .await;
            return;
        }

        if let Some(body) = rest.strip_prefix("capture ") {
            let name = body.trim();
            if name.is_empty() {
                self.emit(
                    &sink,
                    vec![SearchItemDto {
                        id: "resume:capture-help".into(),
                        module_id: MODULE_ID.into(),
                        title: "Capture from current directory".into(),
                        subtitle: Some("resume capture <name>".into()),
                        kind: "not_configured".into(),
                        score: 1.0,
                        primary_action_id: "noop".into(),
                        primary_action_label: "OK".into(),
                        ui_intent: None,
                        action_payload: None,
                    ..Default::default()
                    }],
                )
                .await;
                return;
            }
            if let Ok(key) = normalize_resume_name(name) {
                self.emit(&sink, vec![Self::capture_preview_row(&key)]).await;
            }
            return;
        }

        if let Some(body) = rest.strip_prefix("delete ") {
            let name = body.trim();
            if let Ok(key) = normalize_resume_name(name) {
                match self.store.get(&key) {
                    Ok(Some(_)) => self.emit(&sink, vec![Self::delete_preview_row(&key)]).await,
                    Ok(None) => self.emit(&sink, vec![Self::not_found_row(&key)]).await,
                    Err(err) => {
                        self.emit(&sink, vec![Self::unavailable_row(err.to_string())])
                            .await;
                    }
                }
            }
            return;
        }

        if let Some(body) = rest.strip_prefix("edit ") {
            let name = body.trim();
            if let Ok(key) = normalize_resume_name(name) {
                match self.store.get(&key) {
                    Ok(Some(ctx)) => {
                        let mut dto = Self::dto_for_context(&ctx, 10.0);
                        dto.primary_action_id = "noop".into();
                        dto.primary_action_label = "View".into();
                        dto.subtitle = Some(format!(
                            "Ctrl-k to edit fields · or resume set {key} <field> <value>"
                        ));
                        self.emit(&sink, vec![dto]).await;
                    }
                    Ok(None) => self.emit(&sink, vec![Self::not_found_row(&key)]).await,
                    Err(err) => {
                        self.emit(&sink, vec![Self::unavailable_row(err.to_string())])
                            .await;
                    }
                }
            }
            return;
        }

        if let Some(body_norm) = rest.strip_prefix("set ") {
            let body_raw = rest_raw
                .strip_prefix("set ")
                .or_else(|| rest_raw.strip_prefix("Set "))
                .unwrap_or(rest_raw)
                .trim();
            let mut parts = body_raw.splitn(3, char::is_whitespace);
            let name = parts.next().unwrap_or("").trim();
            let field = parts.next().unwrap_or("").trim().to_ascii_lowercase();
            let value = parts.next().unwrap_or("").trim();
            if name.is_empty() || field.is_empty() {
                self.emit(
                    &sink,
                    vec![SearchItemDto {
                        id: "resume:set-help".into(),
                        module_id: MODULE_ID.into(),
                        title: "Set a field".into(),
                        subtitle: Some(
                            "resume set <name> <field> <value>  (documents/notes/recipes use | separators)"
                                .into(),
                        ),
                        kind: "not_configured".into(),
                        score: 1.0,
                        primary_action_id: "noop".into(),
                        primary_action_label: "OK".into(),
                        ui_intent: None,
                        action_payload: None,
                    ..Default::default()
                    }],
                )
                .await;
                return;
            }
            if value.is_empty() {
                // Incomplete — show seed-style hint row.
                if let Ok(key) = normalize_resume_name(name) {
                    self.emit(
                        &sink,
                        vec![SearchItemDto {
                            id: format!("resume:set-incomplete:{key}:{field}"),
                            module_id: MODULE_ID.into(),
                            title: format!("Set {field} on “{key}”"),
                            subtitle: Some(format!("type value after: resume set {key} {field} ")),
                            kind: "not_configured".into(),
                            score: 1.0,
                            primary_action_id: "noop".into(),
                            primary_action_label: "OK".into(),
                            ui_intent: None,
                            action_payload: None,
                        ..Default::default()
                        }],
                    )
                    .await;
                }
                return;
            }
            let _ = body_norm;
            if let Ok(key) = normalize_resume_name(name) {
                self.emit(&sink, vec![Self::set_preview_row(&key, &field, value)])
                    .await;
            }
            return;
        }

        // `resume <name>` — restore lookup
        let name = rest.split_whitespace().next().unwrap_or("").trim();
        if let Ok(key) = normalize_resume_name(name) {
            match self.store.get(&key) {
                Ok(Some(ctx)) => self.emit(&sink, vec![Self::dto_for_context(&ctx, 10.0)]).await,
                Ok(None) => self.emit(&sink, vec![Self::not_found_row(&key)]).await,
                Err(err) => {
                    self.emit(&sink, vec![Self::unavailable_row(err.to_string())])
                        .await;
                }
            }
        }
    }

    async fn actions(&self, result: &SearchItem) -> Vec<ActionDescriptor> {
        if result.id.as_str() == "resume:unavailable" {
            return vec![
                action("rebuild_store", "Rebuild empty store", ActionRisk::Confirm, true),
                action("noop", "Dismiss", ActionRisk::Safe, false),
            ];
        }
        if result.kind == "resume" || Self::context_name_from_id(result.id.as_str()).is_some() {
            let mut actions = vec![
                action("resume_entry", "Resume entry", ActionRisk::Safe, false),
                action("open_project", "Open project", ActionRisk::Safe, false),
                action("open_worktree", "Open worktree", ActionRisk::Safe, false),
                action("open_docs", "Open docs", ActionRisk::Safe, false),
                action("open_editor", "Open editor", ActionRisk::Safe, false),
                action("open_notes", "Open notes", ActionRisk::Safe, false),
                action("show_recipes", "Show recipes", ActionRisk::Safe, false),
                action(
                    "open_terminal",
                    "Open terminal here",
                    ActionRisk::Safe,
                    false,
                ),
                action(
                    "connect_ssh",
                    "Connect SSH",
                    ActionRisk::Confirm,
                    true,
                ),
            ];
            if let Some(name) = Self::resolve_name_from_item(result) {
                if let Ok(Some(ctx)) = self.store.get(&name) {
                    for (i, recipe) in ctx.recipes.iter().enumerate() {
                        let label = match &recipe.command {
                            Some(cmd) => format!("Run recipe: {} ({cmd})", recipe.name),
                            None => format!("Run recipe: {}", recipe.name),
                        };
                        actions.push(action(
                            &format!("run_recipe:{i}"),
                            &label,
                            ActionRisk::Confirm,
                            true,
                        ));
                    }
                }
            }
            actions.extend([
                action(
                    "seed_set_project_path",
                    "Edit project path",
                    ActionRisk::Safe,
                    false,
                ),
                action(
                    "seed_set_git_branch",
                    "Edit branch",
                    ActionRisk::Safe,
                    false,
                ),
                action(
                    "seed_set_ssh_host",
                    "Edit SSH host",
                    ActionRisk::Safe,
                    false,
                ),
                action("seed_set_editor", "Edit editor", ActionRisk::Safe, false),
                action(
                    "seed_set_terminal_cwd",
                    "Edit terminal cwd",
                    ActionRisk::Safe,
                    false,
                ),
                action("capture", "Capture from cwd", ActionRisk::Safe, false),
                action("delete", "Delete", ActionRisk::Destructive, true),
            ]);
            return actions;
        }
        match result.primary_action.id.as_str() {
            "save" => vec![action("save", "Save", ActionRisk::Safe, false)],
            "capture" => vec![action("capture", "Capture & save", ActionRisk::Safe, false)],
            "delete" => vec![action("delete", "Delete", ActionRisk::Destructive, true)],
            "set_field" => vec![action("set_field", "Save field", ActionRisk::Safe, false)],
            "rebuild_store" => {
                vec![action(
                    "rebuild_store",
                    "Rebuild empty store",
                    ActionRisk::Confirm,
                    true,
                )]
            }
            _ => vec![action("noop", "OK", ActionRisk::Safe, false)],
        }
    }

    async fn preview(&self, result: &SearchItem) -> Option<String> {
        let name = Self::resolve_name_from_item(result)?;
        let ctx = self.store.get(&name).ok().flatten()?;
        Some(Self::preview_text(&ctx))
    }

    async fn perform(&self, request: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        let action_id = request.action.id.as_str();
        if request.action.confirmation && !request.confirmation {
            return ActionOutcome::Cancelled;
        }

        match action_id {
            "noop" | "seed_config" => ActionOutcome::Success { message: None },
            "rebuild_store" => match self.store.rebuild_empty() {
                Ok(()) => {
                    self.set_store_error(None).await;
                    ActionOutcome::Success {
                        message: Some("resume store rebuilt (empty)".into()),
                    }
                }
                Err(err) => ActionOutcome::Failed {
                    kind: FailureKind::Unavailable {
                        reason: err.to_string(),
                        retryable: true,
                    },
                },
            },
            "save" => {
                let Some(name) = Self::resolve_name_from_item(&request.result) else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "name".into(),
                            message: "missing name".into(),
                        },
                    };
                };
                self.perform_save(&name).await
            }
            "capture" => {
                let Some(name) = Self::resolve_name_from_item(&request.result) else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "name".into(),
                            message: "missing name".into(),
                        },
                    };
                };
                self.perform_capture(&name, cancel).await
            }
            "set_field" => {
                let payload = request.result.action_payload.clone().unwrap_or_default();
                let name = payload
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let field = payload
                    .get("field")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let value = payload
                    .get("value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                self.perform_set(name, field, value).await
            }
            "delete" => {
                let Some(name) = Self::resolve_name_from_item(&request.result) else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "name".into(),
                            message: "missing name".into(),
                        },
                    };
                };
                match self.store.delete(&name) {
                    Ok(()) => ActionOutcome::Success {
                        message: Some(format!("deleted “{name}”")),
                    },
                    Err(err) => ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: err.to_string(),
                            retryable: true,
                        },
                    },
                }
            }
            "resume_entry" => {
                let Some(name) = Self::resolve_name_from_item(&request.result) else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::NotFound {
                            entity: "resume".into(),
                        },
                    };
                };
                self.resume_entry(&name, cancel).await
            }
            "open_project" | "open_worktree" | "open_docs" | "open_notes" => {
                let Some(name) = Self::resolve_name_from_item(&request.result) else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::NotFound {
                            entity: "resume".into(),
                        },
                    };
                };
                let ctx = match self.store.get(&name) {
                    Ok(Some(c)) => c,
                    Ok(None) => {
                        return ActionOutcome::Failed {
                            kind: FailureKind::NotFound {
                                entity: format!("resume:{name}"),
                            },
                        };
                    }
                    Err(err) => {
                        return ActionOutcome::Failed {
                            kind: FailureKind::Unavailable {
                                reason: err.to_string(),
                                retryable: true,
                            },
                        };
                    }
                };
                let mut reports = Vec::new();
                match action_id {
                    "open_project" => {
                        self.open_path_step(
                            "project",
                            ctx.project_path.as_deref(),
                            &cancel,
                            &mut reports,
                        )
                        .await;
                    }
                    "open_worktree" => {
                        self.open_path_step(
                            "worktree",
                            ctx.worktree_path.as_deref(),
                            &cancel,
                            &mut reports,
                        )
                        .await;
                    }
                    "open_docs" => {
                        if ctx.documents.is_empty() {
                            reports.push("docs: not configured".into());
                        }
                        for doc in &ctx.documents {
                            self.open_path_step("doc", Some(doc), &cancel, &mut reports)
                                .await;
                        }
                    }
                    "open_notes" => {
                        if ctx.notes.is_empty() {
                            reports.push("notes: not configured".into());
                        }
                        for note in &ctx.notes {
                            self.open_path_step("note", Some(note), &cancel, &mut reports)
                                .await;
                        }
                    }
                    _ => {}
                }
                ActionOutcome::Success {
                    message: Some(reports.join(" · ")),
                }
            }
            "open_editor" => {
                let Some(name) = Self::resolve_name_from_item(&request.result) else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::NotFound {
                            entity: "resume".into(),
                        },
                    };
                };
                let ctx = match self.store.get(&name) {
                    Ok(Some(c)) => c,
                    Ok(None) => {
                        return ActionOutcome::Failed {
                            kind: FailureKind::NotFound {
                                entity: format!("resume:{name}"),
                            },
                        };
                    }
                    Err(err) => {
                        return ActionOutcome::Failed {
                            kind: FailureKind::Unavailable {
                                reason: err.to_string(),
                                retryable: true,
                            },
                        };
                    }
                };
                let editor = ctx.editor.clone().unwrap_or(ResumeEditor::Default);
                let path = ctx
                    .editor_project_path
                    .as_ref()
                    .or(ctx.project_path.as_ref())
                    .cloned();
                let Some(path) = path else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::NotConfigured {
                            remediation: format!(
                                "set editor_project_path: resume set {name} editor_project_path <path>"
                            ),
                        },
                    };
                };
                let p = PathBuf::from(&path);
                if !p.exists() {
                    return ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: format!("path missing: {path}"),
                            retryable: false,
                        },
                    };
                }
                match await_unless_cancelled(&cancel, self.editor.open(editor, &p)).await {
                    None => ActionOutcome::Cancelled,
                    Some(Ok(())) => ActionOutcome::Success {
                        message: Some(format!("opened editor at {path}")),
                    },
                    Some(Err(err)) => ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: err.to_string(),
                            retryable: false,
                        },
                    },
                }
            }
            "open_terminal" => {
                let Some(name) = Self::resolve_name_from_item(&request.result) else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::NotFound {
                            entity: "resume".into(),
                        },
                    };
                };
                let ctx = match self.store.get(&name) {
                    Ok(Some(c)) => c,
                    Ok(None) => {
                        return ActionOutcome::Failed {
                            kind: FailureKind::NotFound {
                                entity: format!("resume:{name}"),
                            },
                        };
                    }
                    Err(err) => {
                        return ActionOutcome::Failed {
                            kind: FailureKind::Unavailable {
                                reason: err.to_string(),
                                retryable: true,
                            },
                        };
                    }
                };
                let cwd = ctx
                    .terminal_cwd
                    .as_ref()
                    .or(ctx.project_path.as_ref())
                    .cloned();
                let Some(cwd) = cwd else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::NotConfigured {
                            remediation: format!(
                                "set terminal_cwd: resume set {name} terminal_cwd <path>"
                            ),
                        },
                    };
                };
                let p = PathBuf::from(&cwd);
                if !p.is_dir() {
                    return ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: format!("cwd missing: {cwd}"),
                            retryable: false,
                        },
                    };
                }
                match await_unless_cancelled(&cancel, self.editor.open_terminal(&p)).await {
                    None => ActionOutcome::Cancelled,
                    Some(Ok(())) => ActionOutcome::Success {
                        message: Some(format!("opened Terminal at {cwd}")),
                    },
                    Some(Err(err)) => ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: err.to_string(),
                            retryable: false,
                        },
                    },
                }
            }
            "show_recipes" => {
                let Some(name) = Self::resolve_name_from_item(&request.result) else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::NotFound {
                            entity: "resume".into(),
                        },
                    };
                };
                let ctx = match self.store.get(&name) {
                    Ok(Some(c)) => c,
                    Ok(None) => {
                        return ActionOutcome::Failed {
                            kind: FailureKind::NotFound {
                                entity: format!("resume:{name}"),
                            },
                        };
                    }
                    Err(err) => {
                        return ActionOutcome::Failed {
                            kind: FailureKind::Unavailable {
                                reason: err.to_string(),
                                retryable: true,
                            },
                        };
                    }
                };
                if ctx.recipes.is_empty() {
                    return ActionOutcome::Success {
                        message: Some("no recipes saved (not run on resume)".into()),
                    };
                }
                let lines: Vec<_> = ctx
                    .recipes
                    .iter()
                    .map(|r| match &r.command {
                        Some(cmd) => format!("{}: {cmd}", r.name),
                        None => r.name.clone(),
                    })
                    .collect();
                ActionOutcome::Success {
                    message: Some(format!("recipes (not executed): {}", lines.join(" · "))),
                }
            }
            "connect_ssh" => {
                let Some(name) = Self::resolve_name_from_item(&request.result) else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::NotFound {
                            entity: "resume".into(),
                        },
                    };
                };
                let ctx = match self.store.get(&name) {
                    Ok(Some(c)) => c,
                    Ok(None) => {
                        return ActionOutcome::Failed {
                            kind: FailureKind::NotFound {
                                entity: format!("resume:{name}"),
                            },
                        };
                    }
                    Err(err) => {
                        return ActionOutcome::Failed {
                            kind: FailureKind::Unavailable {
                                reason: err.to_string(),
                                retryable: true,
                            },
                        };
                    }
                };
                let Some(host) = ctx.ssh_host.filter(|h| !h.is_empty()) else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::NotConfigured {
                            remediation: format!(
                                "set ssh host: resume set {name} ssh_host <alias>"
                            ),
                        },
                    };
                };
                ActionOutcome::InteractiveTerminal {
                    program: "ssh".into(),
                    args: ssh_connect_args(&host),
                    record_alias: Some(host),
                }
            }
            id if id.starts_with("run_recipe:") => {
                let Some(name) = Self::resolve_name_from_item(&request.result) else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::NotFound {
                            entity: "resume".into(),
                        },
                    };
                };
                let idx: usize = id
                    .strip_prefix("run_recipe:")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(usize::MAX);
                let ctx = match self.store.get(&name) {
                    Ok(Some(c)) => c,
                    Ok(None) => {
                        return ActionOutcome::Failed {
                            kind: FailureKind::NotFound {
                                entity: format!("resume:{name}"),
                            },
                        };
                    }
                    Err(err) => {
                        return ActionOutcome::Failed {
                            kind: FailureKind::Unavailable {
                                reason: err.to_string(),
                                retryable: true,
                            },
                        };
                    }
                };
                match Self::recipe_plan(&ctx, idx) {
                    Ok(plan) => ActionOutcome::InteractiveRecipeRun {
                        plan: Box::new(plan),
                    },
                    Err(kind) => ActionOutcome::Failed { kind },
                }
            }
            id if id.starts_with("seed_set_") => ActionOutcome::Success {
                message: Some("type the value in the prompt".into()),
            },
            other => ActionOutcome::Failed {
                kind: FailureKind::NotFound {
                    entity: format!("action:{other}"),
                },
            },
        }
    }

    async fn teardown(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_application::{
        FakeGitInfo, FakeOpenEditor, FakeOpenPath, GitSnapshot, MemoryResumeContextsRepository,
        OpenEditorError,
    };
    use luma_test_support::collect_search_items;

    fn module() -> (
        ResumeModule,
        Arc<MemoryResumeContextsRepository>,
        Arc<FakeOpenPath>,
        Arc<FakeOpenEditor>,
    ) {
        let store = Arc::new(MemoryResumeContextsRepository::new());
        let opener = Arc::new(FakeOpenPath::new());
        let editor = Arc::new(FakeOpenEditor::new());
        let git = Arc::new(FakeGitInfo::with_error(GitInfoError::NotARepo));
        let m = ResumeModule::with_deps(
            store.clone(),
            opener.clone(),
            editor.clone(),
            git,
        );
        (m, store, opener, editor)
    }

    fn sample(name: &str) -> ResumeContext {
        let mut ctx = blank_context(name).unwrap();
        ctx.project_path = Some("/tmp/proj with spaces".into());
        ctx.git_branch = Some("main".into());
        ctx.worktree_path = Some("/tmp/proj with spaces".into());
        ctx.ssh_host = Some("prod".into());
        ctx.recipes = vec![ResumeRecipeRef {
            name: "check".into(),
            command: Some("cargo test".into()),
        }];
        ctx.documents = vec!["/tmp/proj with spaces/README.md".into()];
        ctx.editor = Some(ResumeEditor::Cursor);
        ctx.editor_project_path = Some("/tmp/proj with spaces".into());
        ctx.notes = vec!["/tmp/中文笔记.md".into()];
        ctx.terminal_cwd = Some("/tmp/proj with spaces".into());
        ctx
    }

    #[tokio::test]
    async fn list_empty_and_sorted() {
        let (m, store, _, _) = module();
        let items = collect_search_items(&m, Query::parse("resume ", 20)).await;
        assert_eq!(items[0].kind, "not_configured");

        let mut older = sample("older");
        older.updated_at = "2020-01-01T00:00:00Z".into();
        let mut newer = sample("newer");
        newer.updated_at = "2024-01-01T00:00:00Z".into();
        store.upsert(older).unwrap();
        store.upsert(newer).unwrap();
        let items = collect_search_items(&m, Query::parse("resume list", 20)).await;
        assert_eq!(items[0].id.as_str(), "resume:newer");
        store.mark_resumed("older").unwrap();
        let items = collect_search_items(&m, Query::parse("resume", 20)).await;
        assert_eq!(items[0].id.as_str(), "resume:older");
    }

    #[tokio::test]
    async fn name_lookup_missing_is_friendly() {
        let (m, _, _, _) = module();
        let items = collect_search_items(&m, Query::parse("resume missingname", 20)).await;
        assert!(items[0].title.contains("No resume context"));
        assert_ne!(items[0].kind, "resume");
    }

    #[tokio::test]
    async fn save_edit_delete_and_set_field() {
        let (m, store, _, _) = module();
        let items = collect_search_items(&m, Query::parse("resume save luma", 20)).await;
        assert_eq!(items[0].primary_action.id.as_str(), "save");
        let outcome = m
            .perform(
                ActionRequest {
                    result: items[0].clone(),
                    action: items[0].primary_action.clone(),
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(outcome, ActionOutcome::Success { .. }));
        assert!(store.get("luma").unwrap().is_some());

        let items = collect_search_items(
            &m,
            Query::parse("resume set luma project_path /tmp/luma-proj", 20),
        )
        .await;
        let _ = std::fs::create_dir_all("/tmp/luma-proj");
        let outcome = m
            .perform(
                ActionRequest {
                    result: items[0].clone(),
                    action: items[0].primary_action.clone(),
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(outcome, ActionOutcome::Success { .. }));
        let ctx = store.get("luma").unwrap().unwrap();
        assert!(ctx.project_path.as_deref().unwrap().contains("luma-proj"));

        let items = collect_search_items(&m, Query::parse("resume delete luma", 20)).await;
        assert!(items[0].primary_action.confirmation);
        let outcome = m
            .perform(
                ActionRequest {
                    result: items[0].clone(),
                    action: items[0].primary_action.clone(),
                    confirmation: true,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(outcome, ActionOutcome::Success { .. }));
        assert!(store.get("luma").unwrap().is_none());
    }

    #[tokio::test]
    async fn resume_entry_does_not_run_recipes_or_ssh() {
        let (m, store, opener, editor) = module();
        // Use paths that may not exist — resume should report failures but not execute recipes.
        store.upsert(sample("luma")).unwrap();
        let items = collect_search_items(&m, Query::parse("resume luma", 20)).await;
        let outcome = m
            .perform(
                ActionRequest {
                    result: items[0].clone(),
                    action: items[0].primary_action.clone(),
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        match outcome {
            ActionOutcome::Success { message } => {
                let msg = message.unwrap_or_default();
                assert!(msg.contains("recipes:"));
                assert!(msg.contains("not run"));
                assert!(msg.contains("ssh:"));
                assert!(msg.contains("not connected"));
            }
            other => panic!("unexpected {other:?}"),
        }
        assert!(matches!(
            outcome_actions_confirm(&m, &items[0]).await,
            true
        ));
        // No InteractiveRecipeRun from primary resume.
        let _ = opener;
        let _ = editor;
        let updated = store.get("luma").unwrap().unwrap();
        // last_resumed may update even when opens fail if project_path is set
        assert!(updated.last_resumed_at.is_some());
    }

    async fn outcome_actions_confirm(m: &ResumeModule, item: &SearchItem) -> bool {
        let actions = m.actions(item).await;
        let ssh = actions.iter().find(|a| a.id.as_str() == "connect_ssh").unwrap();
        let recipe = actions
            .iter()
            .find(|a| a.id.as_str().starts_with("run_recipe:"))
            .unwrap();
        ssh.confirmation && recipe.confirmation
    }

    #[tokio::test]
    async fn connect_ssh_and_run_recipe_require_confirm_and_build_plan() {
        let (m, store, _, _) = module();
        let mut ctx = sample("srv");
        ctx.terminal_cwd = Some("/tmp".into());
        store.upsert(ctx).unwrap();
        let items = collect_search_items(&m, Query::parse("resume srv", 20)).await;
        let actions = m.actions(&items[0]).await;
        let ssh = actions.iter().find(|a| a.id.as_str() == "connect_ssh").unwrap();
        assert!(ssh.confirmation);
        let cancelled = m
            .perform(
                ActionRequest {
                    result: items[0].clone(),
                    action: ssh.clone(),
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(cancelled, ActionOutcome::Cancelled));
        let connected = m
            .perform(
                ActionRequest {
                    result: items[0].clone(),
                    action: ssh.clone(),
                    confirmation: true,
                },
                CancellationToken::new(),
            )
            .await;
        match connected {
            ActionOutcome::InteractiveTerminal { program, args, .. } => {
                assert_eq!(program, "ssh");
                assert_eq!(args, vec!["prod"]);
            }
            other => panic!("{other:?}"),
        }

        let run = actions
            .iter()
            .find(|a| a.id.as_str() == "run_recipe:0")
            .unwrap();
        assert!(run.confirmation);
        let plan = m
            .perform(
                ActionRequest {
                    result: items[0].clone(),
                    action: run.clone(),
                    confirmation: true,
                },
                CancellationToken::new(),
            )
            .await;
        match plan {
            ActionOutcome::InteractiveRecipeRun { plan } => {
                assert_eq!(plan.steps[0].program, "cargo");
                assert_eq!(plan.steps[0].args, vec!["test"]);
            }
            other => panic!("{other:?}"),
        }
    }

    #[tokio::test]
    async fn editor_unavailable_continues_other_steps() {
        let store = Arc::new(MemoryResumeContextsRepository::new());
        let opener = Arc::new(FakeOpenPath::new());
        let editor = Arc::new(FakeOpenEditor::with_failure(OpenEditorError::Unavailable(
            "Cursor is not installed".into(),
        )));
        let git = Arc::new(FakeGitInfo::with_error(GitInfoError::NotARepo));
        let m = ResumeModule::with_deps(store.clone(), opener.clone(), editor.clone(), git);
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("proj");
        std::fs::create_dir_all(&project).unwrap();
        let mut ctx = blank_context("luma").unwrap();
        ctx.project_path = Some(project.display().to_string());
        ctx.editor = Some(ResumeEditor::Cursor);
        ctx.editor_project_path = Some(project.display().to_string());
        store.upsert(ctx).unwrap();
        let items = collect_search_items(&m, Query::parse("resume luma", 20)).await;
        let outcome = m
            .perform(
                ActionRequest {
                    result: items[0].clone(),
                    action: items[0].primary_action.clone(),
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        match outcome {
            ActionOutcome::Success { message } => {
                let msg = message.unwrap();
                assert!(msg.contains("project: ok"));
                assert!(msg.contains("editor: failed"));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(opener.open_count.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn capture_uses_git_when_available() {
        let store = Arc::new(MemoryResumeContextsRepository::new());
        let opener = Arc::new(FakeOpenPath::new());
        let editor = Arc::new(FakeOpenEditor::new());
        let git = Arc::new(FakeGitInfo::with_snapshot(GitSnapshot {
            repo_root: PathBuf::from("/repo/root"),
            branch: Some("feature".into()),
            worktree_path: PathBuf::from("/repo/root"),
        }));
        let m = ResumeModule::with_deps(store.clone(), opener, editor, git);
        let items = collect_search_items(&m, Query::parse("resume capture work", 20)).await;
        let outcome = m
            .perform(
                ActionRequest {
                    result: items[0].clone(),
                    action: items[0].primary_action.clone(),
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(outcome, ActionOutcome::Success { .. }));
        let ctx = store.get("work").unwrap().unwrap();
        assert_eq!(ctx.project_path.as_deref(), Some("/repo/root"));
        assert_eq!(ctx.git_branch.as_deref(), Some("feature"));
    }

    #[tokio::test]
    async fn corrupt_store_shows_unavailable() {
        let store = Arc::new(MemoryResumeContextsRepository::new());
        store.fail_with("corrupt resume store at /tmp/x: bad json");
        let m = ResumeModule::with_deps(
            store.clone(),
            Arc::new(FakeOpenPath::new()),
            Arc::new(FakeOpenEditor::new()),
            Arc::new(FakeGitInfo::new()),
        );
        let items = collect_search_items(&m, Query::parse("resume ", 20)).await;
        assert_eq!(items[0].kind, "unavailable");
        assert_eq!(items[0].primary_action.id.as_str(), "rebuild_store");
    }
}
