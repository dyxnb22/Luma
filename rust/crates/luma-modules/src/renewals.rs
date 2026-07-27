use async_trait::async_trait;
use chrono::{Datelike, Days, Months, NaiveDate};
use luma_application::{
    ActionOutcome, ActionRequest, ClockPort, LumaModule, ModuleManifest, ModuleState, NewRenewal,
    RenewalEntry, RenewalPaidUpdate, RenewalsRepoError, RenewalsRepository, SearchMode, SearchSink,
    WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{ActionDescriptorDto, Event, SearchItemDto};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const MODULE_ID: &str = "luma.renewals";
const MAX_COMMAND_CHARS: usize = 4_096;
const MAX_NAME_CHARS: usize = 160;
const MAX_CATEGORY_CHARS: usize = 64;
const MAX_URL_CHARS: usize = 2_048;
const MAX_NOTE_CHARS: usize = 2_000;

pub struct RenewalsModule {
    manifest: ModuleManifest,
    repository: Arc<dyn RenewalsRepository>,
    clock: Arc<dyn ClockPort>,
}

impl RenewalsModule {
    /// Canonical command discovery owned by this module, including unavailable fallbacks.
    pub fn command_specs() -> Vec<luma_application::CommandSpec> {
        vec![
                        crate::ux::command_spec(
                            "/renew [upcoming|due|30d|query]",
                            "List or search active renewals",
                            "/renew ",
                            Some("/renew 30d"),
                        ),
                        crate::ux::command_spec(
                            "/renew add <name> | <YYYY-MM-DD> | <amount currency> | <cadence> [| category | auto | url | note]",
                            "Add a renewal",
                            "/renew add ",
                            Some("/renew add Music | 2026-08-01 | 10 USD | monthly"),
                        ),
                        crate::ux::command_spec(
                            "/renew edit <id> <all fields>",
                            "Replace an existing renewal using the add field format",
                            "/renew edit ",
                            None,
                        ),
                        crate::ux::command_spec(
                            "/renew paid <id>",
                            "Mark a renewal paid and advance its due date",
                            "/renew paid ",
                            Some("/renew paid 1"),
                        ),
                        crate::ux::command_spec(
                            "/renew cancel <id>",
                            "Cancel an active renewal after confirmation",
                            "/renew cancel ",
                            Some("/renew cancel 1"),
                        ),
                        crate::ux::command_spec(
                            "/renew delete <id>",
                            "Delete renewal metadata after confirmation",
                            "/renew delete ",
                            Some("/renew delete 1"),
                        ),
                        crate::ux::command_spec(
                            "/renew backup",
                            "Back up the renewals ledger",
                            "/renew backup",
                            None,
                        ),
                    ]
    }

    pub fn with_deps(repository: Arc<dyn RenewalsRepository>, clock: Arc<dyn ClockPort>) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new(MODULE_ID),
                display_name: "Renewals".into(),
                triggers: vec!["renew".into(), "renewals".into()],
                default_enabled: true,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
                workbench: luma_application::WorkbenchMeta {
                    glyph: Some("R".into()),
                    suggested_query: Some("/renew ".into()),
                    empty_hint: Some("/renew add NAME | DATE | AMOUNT CURRENCY | monthly".into()),
                    supports_browse: false,
                    commands: Self::command_specs(),
                },
            },
            repository,
            clock,
        }
    }

    async fn command_item(
        &self,
        rest: &str,
        cancel: &CancellationToken,
    ) -> Result<Option<SearchItemDto>, CommandError> {
        if rest.chars().count() > MAX_COMMAND_CHARS {
            return Err(CommandError::Invalid(
                "command exceeds the 4096-character limit".into(),
            ));
        }
        if rest.eq_ignore_ascii_case("backup") {
            return Ok(Some(command_item(
                "renew:backup",
                "Backup renewals",
                "Create a SQLite snapshot in LumaNext/backups/",
                "backup",
                "Backup",
                ActionRisk::Safe,
                false,
                None,
            )));
        }
        if let Some(body) = strip_subcommand(rest, "add") {
            let draft = parse_draft(body)?;
            return Ok(Some(draft_command_item("add", None, draft, None)));
        }
        if let Some(body) = strip_subcommand(rest, "edit") {
            let (id, fields) = split_id_and_rest(body)?;
            let draft = parse_draft(fields)?;
            let current = self.resolve_for_search(id, cancel)?;
            return Ok(Some(draft_command_item(
                "edit",
                Some(id),
                draft,
                Some(current.updated_at),
            )));
        }
        for command in ["paid", "cancel", "delete"] {
            if let Some(body) = strip_subcommand(rest, command) {
                let id = parse_bare_id(body)?;
                let current = self.resolve_for_search(id, cancel)?;
                let (label, risk, confirmation) = match command {
                    "paid" => ("Mark paid", ActionRisk::Safe, false),
                    "cancel" => ("Cancel renewal", ActionRisk::Confirm, true),
                    "delete" => ("Delete renewal", ActionRisk::Destructive, true),
                    _ => unreachable!(),
                };
                return Ok(Some(command_item(
                    &format!("renew:{command}:{id}"),
                    &format!("{label}: {}", current.name),
                    &format!(
                        "{} · due {}",
                        format_amount(current.amount_minor, current.currency.as_deref()),
                        current.next_due_date
                    ),
                    command,
                    label,
                    risk,
                    confirmation,
                    Some(identity_payload(&current)),
                )));
            }
        }
        Ok(None)
    }

    fn resolve_for_search(
        &self,
        id: i64,
        cancel: &CancellationToken,
    ) -> Result<RenewalEntry, CommandError> {
        if cancel.is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let entry = self
            .repository
            .get(id)
            .map_err(CommandError::Repository)?
            .ok_or_else(|| CommandError::Invalid(format!("renewal {id} was not found")))?;
        if cancel.is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        Ok(entry)
    }

    fn fresh_identity(
        &self,
        result: &SearchItem,
        cancel: &CancellationToken,
    ) -> Result<RenewalEntry, ActionOutcome> {
        let payload = result
            .action_payload
            .as_ref()
            .ok_or_else(|| invalid_input("payload"))?;
        let id = payload
            .get("id")
            .and_then(|value| value.as_i64())
            .ok_or_else(|| invalid_input("renewal id"))?;
        let expected_updated_at = payload
            .get("updated_at")
            .and_then(|value| value.as_str())
            .ok_or_else(|| invalid_input("renewal identity"))?;
        if cancel.is_cancelled() {
            return Err(ActionOutcome::Cancelled);
        }
        let current = self
            .repository
            .get(id)
            .map_err(repo_outcome)?
            .ok_or_else(|| ActionOutcome::Failed {
                kind: FailureKind::NotFound {
                    entity: format!("renewal:{id}"),
                },
            })?;
        if cancel.is_cancelled() {
            return Err(ActionOutcome::Cancelled);
        }
        if current.updated_at != expected_updated_at {
            return Err(ActionOutcome::Failed {
                kind: FailureKind::Conflict {
                    reason: "renewal changed; search again before acting".into(),
                },
            });
        }
        Ok(current)
    }
}

#[async_trait]
impl LumaModule for RenewalsModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, ctx: WarmupContext) -> ModuleState {
        if ctx.cancel.is_cancelled() {
            return ModuleState::Cold;
        }
        match self.repository.list() {
            Ok(_) => ModuleState::Ready,
            Err(error) => ModuleState::Failed(error.to_string()),
        }
    }

    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        if cancel.is_cancelled() {
            return;
        }
        let rest = query.rest_raw().trim();
        match self.command_item(rest, &cancel).await {
            Ok(Some(item)) => {
                send_one(&sink, item).await;
                return;
            }
            Err(CommandError::Cancelled) => return,
            Err(CommandError::Invalid(message)) => {
                send_status(
                    &sink,
                    "renew:invalid",
                    "Renewals command is invalid",
                    &message,
                    "command_error",
                )
                .await;
                return;
            }
            Err(CommandError::Repository(error)) => {
                send_repo_error(&sink, error).await;
                return;
            }
            Ok(None) => {}
        }

        let today = match self
            .clock
            .today_ymd()
            .map_err(|error| RenewalsRepoError::Store(error.to_string()))
        {
            Ok(today) => today,
            Err(error) => {
                send_repo_error(&sink, error).await;
                return;
            }
        };
        let today_date = match parse_date(&today) {
            Ok(date) => date,
            Err(message) => {
                send_status(
                    &sink,
                    "renew:unavailable",
                    "Local calendar is unavailable",
                    &message,
                    "unavailable",
                )
                .await;
                return;
            }
        };
        let rows = match self.repository.list() {
            Ok(rows) => rows,
            Err(error) => {
                send_repo_error(&sink, error).await;
                return;
            }
        };
        if cancel.is_cancelled() {
            return;
        }
        let rest_lower = rest.to_lowercase();
        let cutoff = today_date.checked_add_days(Days::new(30));
        let mut visible = rows
            .into_iter()
            .filter(|entry| {
                if entry.status != "active" {
                    return false;
                }
                let Ok(due) = parse_date(&entry.next_due_date) else {
                    return false;
                };
                match rest_lower.as_str() {
                    "" | "upcoming" => true,
                    "due" => due <= today_date,
                    "30d" => cutoff.is_some_and(|cutoff| due >= today_date && due <= cutoff),
                    _ => {
                        entry.name.to_lowercase().contains(&rest_lower)
                            || entry.category.to_lowercase().contains(&rest_lower)
                    }
                }
            })
            .take(query.limit)
            .enumerate()
            .map(|(index, entry)| renewal_item(entry, 80.0 - index as f64 * 0.1))
            .collect::<Vec<_>>();
        if visible.is_empty() {
            visible.push(SearchItemDto {
                id: "renew:empty".into(),
                module_id: MODULE_ID.into(),
                title: if rest_lower == "due" {
                    "No renewals are due".into()
                } else {
                    "No matching active renewals".into()
                },
                subtitle: Some("/renew add NAME | YYYY-MM-DD | AMOUNT CURRENCY | monthly".into()),
                kind: "status".into(),
                score: 0.0,
                primary_action_id: "noop".into(),
                primary_action_label: "OK".into(),
                ..Default::default()
            });
        }
        let _ = sink
            .send(Event::ResultsChunk {
                request_id: String::new(),
                sequence: 1,
                upserts: visible,
                removed_ids: vec![],
            })
            .await;
    }

    async fn actions(&self, result: &SearchItem) -> Vec<ActionDescriptor> {
        match result.kind.as_str() {
            "renewal" => vec![
                safe_action("paid", "Mark paid"),
                confirm_action("cancel", "Cancel renewal", ActionRisk::Confirm),
                confirm_action("delete", "Delete renewal", ActionRisk::Destructive),
            ],
            "renewal_command" => {
                let id = result.primary_action.id.as_str();
                match id {
                    "cancel" => vec![confirm_action(
                        "cancel",
                        "Cancel renewal",
                        ActionRisk::Confirm,
                    )],
                    "delete" => vec![confirm_action(
                        "delete",
                        "Delete renewal",
                        ActionRisk::Destructive,
                    )],
                    "add" | "edit" | "paid" | "backup" => {
                        vec![safe_action(id, &result.primary_action.label)]
                    }
                    _ => vec![safe_action("noop", "OK")],
                }
            }
            _ => vec![safe_action("noop", "OK")],
        }
    }

    async fn perform(&self, request: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        match request.action.id.as_str() {
            "noop" => ActionOutcome::Success { message: None },
            "backup" => {
                if cancel.is_cancelled() {
                    return ActionOutcome::Cancelled;
                }
                match self.repository.backup() {
                    Ok(path) => ActionOutcome::Success {
                        message: Some(format!("backup saved to {}", path.display())),
                    },
                    Err(error) => repo_outcome(error),
                }
            }
            "add" => {
                let draft = match draft_from_payload(request.result.action_payload.as_ref()) {
                    Ok(draft) => draft,
                    Err(error) => return invalid_input(&error),
                };
                let now = match self.clock.now_rfc3339() {
                    Ok(now) => now,
                    Err(error) => return unavailable(error.to_string()),
                };
                if cancel.is_cancelled() {
                    return ActionOutcome::Cancelled;
                }
                match self.repository.insert(&draft.into_new(now)) {
                    Ok(entry) => ActionOutcome::Success {
                        message: Some(format!("added renewal {} ({})", entry.name, entry.id)),
                    },
                    Err(error) => repo_outcome(error),
                }
            }
            "edit" => {
                let current = match self.fresh_identity(&request.result, &cancel) {
                    Ok(current) => current,
                    Err(outcome) => return outcome,
                };
                let draft = match draft_from_payload(request.result.action_payload.as_ref()) {
                    Ok(draft) => draft,
                    Err(error) => return invalid_input(&error),
                };
                let now = match mutation_version(self.clock.as_ref()) {
                    Ok(now) => now,
                    Err(error) => return unavailable(error.to_string()),
                };
                let expected = current.updated_at.clone();
                let replacement = draft.replace(current, now);
                if cancel.is_cancelled() {
                    return ActionOutcome::Cancelled;
                }
                match self.repository.update(&replacement, &expected) {
                    Ok(()) => ActionOutcome::Success {
                        message: Some(format!("updated renewal {}", replacement.name)),
                    },
                    Err(error) => repo_outcome(error),
                }
            }
            "paid" => {
                let current = match self.fresh_identity(&request.result, &cancel) {
                    Ok(current) => current,
                    Err(outcome) => return outcome,
                };
                if current.status != "active" {
                    return ActionOutcome::Failed {
                        kind: FailureKind::Conflict {
                            reason: "only an active renewal can be marked paid".into(),
                        },
                    };
                }
                let (next_due_date, status) = match advance_after_paid(&current) {
                    Ok(next) => next,
                    Err(message) => return invalid_input(&message),
                };
                let now = match mutation_version(self.clock.as_ref()) {
                    Ok(now) => now,
                    Err(error) => return unavailable(error.to_string()),
                };
                let update = RenewalPaidUpdate {
                    id: current.id,
                    expected_due_date: current.next_due_date.clone(),
                    expected_updated_at: current.updated_at.clone(),
                    next_due_date,
                    status,
                    updated_at: now,
                };
                if cancel.is_cancelled() {
                    return ActionOutcome::Cancelled;
                }
                match self.repository.mark_paid(&update) {
                    Ok(()) => ActionOutcome::Success {
                        message: Some(if update.status == "completed" {
                            format!("completed {}", current.name)
                        } else {
                            format!(
                                "marked {} paid; next {}",
                                current.name, update.next_due_date
                            )
                        }),
                    },
                    Err(error) => repo_outcome(error),
                }
            }
            "cancel" => {
                if !request.confirmation {
                    return confirmation_required("cancel renewal");
                }
                let mut current = match self.fresh_identity(&request.result, &cancel) {
                    Ok(current) => current,
                    Err(outcome) => return outcome,
                };
                if current.status != "active" {
                    return ActionOutcome::Failed {
                        kind: FailureKind::Conflict {
                            reason: "only an active renewal can be cancelled".into(),
                        },
                    };
                }
                let expected = current.updated_at.clone();
                current.status = "cancelled".into();
                current.updated_at = match mutation_version(self.clock.as_ref()) {
                    Ok(now) => now,
                    Err(error) => return unavailable(error.to_string()),
                };
                if cancel.is_cancelled() {
                    return ActionOutcome::Cancelled;
                }
                match self.repository.update(&current, &expected) {
                    Ok(()) => ActionOutcome::Success {
                        message: Some(format!("cancelled {}", current.name)),
                    },
                    Err(error) => repo_outcome(error),
                }
            }
            "delete" => {
                if !request.confirmation {
                    return confirmation_required("delete renewal");
                }
                let current = match self.fresh_identity(&request.result, &cancel) {
                    Ok(current) => current,
                    Err(outcome) => return outcome,
                };
                if cancel.is_cancelled() {
                    return ActionOutcome::Cancelled;
                }
                match self.repository.delete(current.id, &current.updated_at) {
                    Ok(()) => ActionOutcome::Success {
                        message: Some(format!("deleted renewal metadata for {}", current.name)),
                    },
                    Err(error) => repo_outcome(error),
                }
            }
            _ => invalid_input("unsupported renewals action"),
        }
    }

    async fn teardown(&self) {}
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RenewalDraft {
    name: String,
    category: String,
    amount_minor: Option<i64>,
    currency: Option<String>,
    cadence_kind: String,
    cadence_value: Option<i64>,
    anchor_month: Option<u32>,
    anchor_day: Option<u32>,
    next_due_date: String,
    auto_renew: bool,
    url: Option<String>,
    note: Option<String>,
}

impl RenewalDraft {
    fn into_new(self, now: String) -> NewRenewal {
        NewRenewal {
            name: self.name,
            category: self.category,
            amount_minor: self.amount_minor,
            currency: self.currency,
            cadence_kind: self.cadence_kind,
            cadence_value: self.cadence_value,
            anchor_month: self.anchor_month,
            anchor_day: self.anchor_day,
            next_due_date: self.next_due_date,
            auto_renew: self.auto_renew,
            status: "active".into(),
            url: self.url,
            note: self.note,
            now,
        }
    }

    fn replace(self, current: RenewalEntry, now: String) -> RenewalEntry {
        RenewalEntry {
            id: current.id,
            name: self.name,
            category: self.category,
            amount_minor: self.amount_minor,
            currency: self.currency,
            cadence_kind: self.cadence_kind,
            cadence_value: self.cadence_value,
            anchor_month: self.anchor_month,
            anchor_day: self.anchor_day,
            next_due_date: self.next_due_date,
            auto_renew: self.auto_renew,
            status: current.status,
            url: self.url,
            note: self.note,
            created_at: current.created_at,
            updated_at: now,
        }
    }
}

fn mutation_version(clock: &dyn ClockPort) -> Result<String, luma_application::ClockError> {
    let timestamp = clock.now_rfc3339()?;
    Ok(format!("{timestamp}#{}", Uuid::new_v4()))
}

#[derive(Debug)]
enum CommandError {
    Invalid(String),
    Repository(RenewalsRepoError),
    Cancelled,
}

fn parse_draft(input: &str) -> Result<RenewalDraft, CommandError> {
    let parts = input.split('|').map(str::trim).collect::<Vec<_>>();
    if !(4..=8).contains(&parts.len()) {
        return Err(CommandError::Invalid(
            "usage: NAME | YYYY-MM-DD | AMOUNT CURRENCY | CADENCE [| CATEGORY | AUTO | URL | NOTE]"
                .into(),
        ));
    }
    let name = bounded_required(parts[0], "name", MAX_NAME_CHARS)?;
    let due = parse_date(parts[1]).map_err(CommandError::Invalid)?;
    let (amount_minor, currency) =
        parse_amount_currency(parts[2]).map_err(CommandError::Invalid)?;
    let (cadence_kind, cadence_value) = parse_cadence(parts[3]).map_err(CommandError::Invalid)?;
    let category = bounded_optional(parts.get(4).copied(), "category", MAX_CATEGORY_CHARS)?
        .unwrap_or_else(|| "other".into());
    let auto_renew = match parts.get(5).copied().unwrap_or("") {
        "" => cadence_kind != "once",
        value if matches!(value.to_ascii_lowercase().as_str(), "yes" | "true" | "on") => true,
        value if matches!(value.to_ascii_lowercase().as_str(), "no" | "false" | "off") => false,
        _ => {
            return Err(CommandError::Invalid(
                "auto-renew must be on/off, yes/no, or true/false".into(),
            ))
        }
    };
    let url = bounded_optional(parts.get(6).copied(), "url", MAX_URL_CHARS)?;
    if let Some(url) = &url {
        if !(url.starts_with("https://") || url.starts_with("http://")) {
            return Err(CommandError::Invalid(
                "URL must use http:// or https://".into(),
            ));
        }
    }
    let note = bounded_optional(parts.get(7).copied(), "note", MAX_NOTE_CHARS)?;
    Ok(RenewalDraft {
        name,
        category,
        amount_minor,
        currency,
        cadence_kind,
        cadence_value,
        anchor_month: Some(due.month()),
        anchor_day: Some(due.day()),
        next_due_date: due.format("%Y-%m-%d").to_string(),
        auto_renew,
        url,
        note,
    })
}

fn bounded_required(value: &str, field: &str, max_chars: usize) -> Result<String, CommandError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(CommandError::Invalid(format!("{field} cannot be empty")));
    }
    if value.chars().count() > max_chars || value.chars().any(char::is_control) {
        return Err(CommandError::Invalid(format!(
            "{field} exceeds its text limit or contains a forbidden control character"
        )));
    }
    Ok(value.into())
}

fn bounded_optional(
    value: Option<&str>,
    field: &str,
    max_chars: usize,
) -> Result<Option<String>, CommandError> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => bounded_required(value, field, max_chars).map(Some),
        None => Ok(None),
    }
}

fn parse_date(value: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .map_err(|_| format!("invalid date {value:?}; expected YYYY-MM-DD"))
}

fn parse_cadence(value: &str) -> Result<(String, Option<i64>), String> {
    let value = value.trim().to_ascii_lowercase();
    match value.as_str() {
        "once" => Ok(("once".into(), None)),
        "monthly" => Ok(("monthly".into(), None)),
        "quarterly" => Ok(("quarterly".into(), None)),
        "yearly" => Ok(("yearly".into(), None)),
        _ => {
            let days = value
                .strip_suffix('d')
                .and_then(|raw| raw.parse::<i64>().ok())
                .filter(|days| (1..=36_500).contains(days))
                .ok_or_else(|| {
                    "cadence must be once, monthly, quarterly, yearly, or 1d..36500d".to_string()
                })?;
            Ok(("custom_days".into(), Some(days)))
        }
    }
}

fn currency_scale(currency: &str) -> Option<u32> {
    match currency {
        "JPY" | "KRW" | "VND" | "CLP" => Some(0),
        "BHD" | "JOD" | "KWD" | "OMR" | "TND" => Some(3),
        "USD" | "EUR" | "GBP" | "CNY" | "HKD" | "AUD" | "CAD" | "SGD" | "NZD" | "CHF" | "SEK"
        | "NOK" | "DKK" | "PLN" | "CZK" | "INR" | "BRL" | "MXN" | "TWD" | "THB" | "MYR" | "PHP"
        | "IDR" | "ZAR" | "AED" | "SAR" => Some(2),
        _ => None,
    }
}

fn parse_amount_currency(value: &str) -> Result<(Option<i64>, Option<String>), String> {
    let value = value.trim();
    if value == "-" || value.eq_ignore_ascii_case("none") {
        return Ok((None, None));
    }
    let tokens = value.split_whitespace().collect::<Vec<_>>();
    if tokens.len() != 2 {
        return Err("amount must be `AMOUNT CURRENCY` or `-`".into());
    }
    let currency = tokens[1].to_ascii_uppercase();
    let scale = currency_scale(&currency)
        .ok_or_else(|| format!("unsupported or ambiguous currency {currency}"))?;
    let amount = tokens[0];
    if amount.starts_with('-') || amount.starts_with('+') || amount.contains(['e', 'E', ',']) {
        return Err("amount must be a non-negative plain decimal".into());
    }
    let mut components = amount.split('.');
    let whole = components.next().unwrap_or("");
    let fraction = components.next();
    if components.next().is_some()
        || whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err("amount must be a non-negative plain decimal".into());
    }
    let fraction = fraction.unwrap_or("");
    if !fraction.bytes().all(|byte| byte.is_ascii_digit()) || fraction.len() > scale as usize {
        return Err(format!(
            "{currency} supports at most {scale} fractional digits"
        ));
    }
    let factor = 10_i64.pow(scale);
    let whole = whole
        .parse::<i64>()
        .map_err(|_| "amount is too large".to_string())?;
    let mut padded = fraction.to_string();
    while padded.len() < scale as usize {
        padded.push('0');
    }
    let minor_fraction = if padded.is_empty() {
        0
    } else {
        padded
            .parse::<i64>()
            .map_err(|_| "amount is invalid".to_string())?
    };
    let minor = whole
        .checked_mul(factor)
        .and_then(|value| value.checked_add(minor_fraction))
        .ok_or_else(|| "amount is too large".to_string())?;
    Ok((Some(minor), Some(currency)))
}

fn advance_after_paid(entry: &RenewalEntry) -> Result<(String, String), String> {
    let current = parse_date(&entry.next_due_date)?;
    if entry.cadence_kind == "once" {
        return Ok((entry.next_due_date.clone(), "completed".into()));
    }
    let anchor_day = entry.anchor_day.unwrap_or(current.day());
    let next = match entry.cadence_kind.as_str() {
        "monthly" => advance_months(current, 1, anchor_day)?,
        "quarterly" => advance_months(current, 3, anchor_day)?,
        "yearly" => {
            let anchor_month = entry.anchor_month.unwrap_or(current.month());
            let year = current
                .year()
                .checked_add(1)
                .ok_or_else(|| "yearly recurrence overflow".to_string())?;
            anchored_date(year, anchor_month, anchor_day)?
        }
        "custom_days" => {
            let days = entry
                .cadence_value
                .filter(|days| (1..=36_500).contains(days))
                .ok_or_else(|| "custom cadence is invalid".to_string())?;
            current
                .checked_add_days(Days::new(days as u64))
                .ok_or_else(|| "custom recurrence overflow".to_string())?
        }
        other => return Err(format!("unsupported cadence {other}")),
    };
    Ok((next.format("%Y-%m-%d").to_string(), "active".into()))
}

fn advance_months(current: NaiveDate, months: u32, anchor_day: u32) -> Result<NaiveDate, String> {
    let first = NaiveDate::from_ymd_opt(current.year(), current.month(), 1)
        .ok_or_else(|| "invalid recurrence date".to_string())?;
    let target = first
        .checked_add_months(Months::new(months))
        .ok_or_else(|| "monthly recurrence overflow".to_string())?;
    anchored_date(target.year(), target.month(), anchor_day)
}

fn anchored_date(year: i32, month: u32, anchor_day: u32) -> Result<NaiveDate, String> {
    let next_month = if month == 12 {
        NaiveDate::from_ymd_opt(
            year.checked_add(1)
                .ok_or_else(|| "recurrence overflow".to_string())?,
            1,
            1,
        )
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .ok_or_else(|| "invalid recurrence month".to_string())?;
    let last_day = next_month
        .pred_opt()
        .ok_or_else(|| "recurrence underflow".to_string())?
        .day();
    NaiveDate::from_ymd_opt(year, month, anchor_day.min(last_day))
        .ok_or_else(|| "invalid anchored recurrence".to_string())
}

fn strip_subcommand<'a>(rest: &'a str, command: &str) -> Option<&'a str> {
    let (head, tail) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
    head.eq_ignore_ascii_case(command).then_some(tail.trim())
}

fn split_id_and_rest(input: &str) -> Result<(i64, &str), CommandError> {
    let (id, rest) = input.split_once(char::is_whitespace).ok_or_else(|| {
        CommandError::Invalid("edit requires an ID followed by all renewal fields".into())
    })?;
    let id = id
        .parse::<i64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| CommandError::Invalid("renewal ID must be a positive integer".into()))?;
    Ok((id, rest.trim()))
}

fn parse_bare_id(input: &str) -> Result<i64, CommandError> {
    if input.split_whitespace().count() != 1 {
        return Err(CommandError::Invalid(
            "command requires exactly one renewal ID".into(),
        ));
    }
    input
        .parse::<i64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| CommandError::Invalid("renewal ID must be a positive integer".into()))
}

fn draft_payload(draft: &RenewalDraft) -> serde_json::Value {
    serde_json::json!({
        "name": draft.name,
        "category": draft.category,
        "amount_minor": draft.amount_minor,
        "currency": draft.currency,
        "cadence_kind": draft.cadence_kind,
        "cadence_value": draft.cadence_value,
        "anchor_month": draft.anchor_month,
        "anchor_day": draft.anchor_day,
        "next_due_date": draft.next_due_date,
        "auto_renew": draft.auto_renew,
        "url": draft.url,
        "note": draft.note,
    })
}

fn draft_from_payload(payload: Option<&serde_json::Value>) -> Result<RenewalDraft, String> {
    let payload = payload.ok_or_else(|| "missing renewal payload".to_string())?;
    let string = |key: &str| {
        payload
            .get(key)
            .and_then(|value| value.as_str())
            .map(str::to_string)
            .ok_or_else(|| format!("missing {key}"))
    };
    let optional_string = |key: &str| {
        payload
            .get(key)
            .and_then(|value| value.as_str())
            .map(str::to_string)
    };
    Ok(RenewalDraft {
        name: string("name")?,
        category: string("category")?,
        amount_minor: payload.get("amount_minor").and_then(|value| value.as_i64()),
        currency: optional_string("currency"),
        cadence_kind: string("cadence_kind")?,
        cadence_value: payload
            .get("cadence_value")
            .and_then(|value| value.as_i64()),
        anchor_month: payload
            .get("anchor_month")
            .and_then(|value| value.as_u64())
            .and_then(|value| u32::try_from(value).ok()),
        anchor_day: payload
            .get("anchor_day")
            .and_then(|value| value.as_u64())
            .and_then(|value| u32::try_from(value).ok()),
        next_due_date: string("next_due_date")?,
        auto_renew: payload
            .get("auto_renew")
            .and_then(|value| value.as_bool())
            .ok_or_else(|| "missing auto_renew".to_string())?,
        url: optional_string("url"),
        note: optional_string("note"),
    })
}

fn identity_payload(entry: &RenewalEntry) -> serde_json::Value {
    serde_json::json!({
        "id": entry.id,
        "updated_at": entry.updated_at,
    })
}

fn draft_command_item(
    action: &str,
    id: Option<i64>,
    draft: RenewalDraft,
    updated_at: Option<String>,
) -> SearchItemDto {
    let mut payload = draft_payload(&draft);
    if let Some(object) = payload.as_object_mut() {
        if let Some(id) = id {
            object.insert("id".into(), serde_json::json!(id));
        }
        if let Some(updated_at) = updated_at {
            object.insert("updated_at".into(), serde_json::json!(updated_at));
        }
    }
    command_item(
        &format!("renew:{action}:{}", id.unwrap_or(0)),
        &format!(
            "{} renewal: {}",
            if action == "add" { "Add" } else { "Edit" },
            draft.name
        ),
        &format!(
            "{} · due {} · {}",
            format_amount(draft.amount_minor, draft.currency.as_deref()),
            draft.next_due_date,
            cadence_label(&draft.cadence_kind, draft.cadence_value)
        ),
        action,
        if action == "add" { "Add" } else { "Save" },
        ActionRisk::Safe,
        false,
        Some(payload),
    )
}

#[allow(clippy::too_many_arguments)]
fn command_item(
    id: &str,
    title: &str,
    subtitle: &str,
    action: &str,
    action_label: &str,
    risk: ActionRisk,
    confirmation: bool,
    payload: Option<serde_json::Value>,
) -> SearchItemDto {
    SearchItemDto {
        id: id.into(),
        module_id: MODULE_ID.into(),
        title: title.into(),
        subtitle: Some(subtitle.into()),
        kind: "renewal_command".into(),
        score: 100.0,
        primary_action_id: action.into(),
        primary_action_label: action_label.into(),
        primary_action_risk: risk,
        primary_action_confirmation: confirmation,
        action_payload: payload,
        ..Default::default()
    }
}

fn renewal_item(entry: RenewalEntry, score: f64) -> SearchItemDto {
    SearchItemDto {
        id: format!("renew:{}", entry.id),
        module_id: MODULE_ID.into(),
        title: entry.name.clone(),
        subtitle: Some(format!(
            "{} · due {} · {} · {}",
            format_amount(entry.amount_minor, entry.currency.as_deref()),
            entry.next_due_date,
            cadence_label(&entry.cadence_kind, entry.cadence_value),
            entry.category
        )),
        kind: "renewal".into(),
        score,
        primary_action_id: "paid".into(),
        primary_action_label: "Mark paid".into(),
        secondary_actions: vec![
            action_dto("cancel", "Cancel renewal", ActionRisk::Confirm, true),
            action_dto("delete", "Delete renewal", ActionRisk::Destructive, true),
        ],
        action_payload: Some(identity_payload(&entry)),
        ..Default::default()
    }
}

fn format_amount(amount_minor: Option<i64>, currency: Option<&str>) -> String {
    let (Some(amount), Some(currency)) = (amount_minor, currency) else {
        return "amount not set".into();
    };
    let scale = currency_scale(currency).unwrap_or(0);
    if scale == 0 {
        return format!("{amount} {currency}");
    }
    let factor = 10_i64.pow(scale);
    format!(
        "{}.{:0width$} {currency}",
        amount / factor,
        amount % factor,
        width = scale as usize
    )
}

fn cadence_label(kind: &str, value: Option<i64>) -> String {
    if kind == "custom_days" {
        format!("every {}d", value.unwrap_or_default())
    } else {
        kind.into()
    }
}

fn action_dto(id: &str, label: &str, risk: ActionRisk, confirmation: bool) -> ActionDescriptorDto {
    ActionDescriptorDto {
        id: id.into(),
        label: label.into(),
        risk,
        confirmation,
    }
}

fn safe_action(id: &str, label: &str) -> ActionDescriptor {
    ActionDescriptor {
        id: ActionId::new(id),
        label: label.into(),
        risk: ActionRisk::Safe,
        confirmation: false,
    }
}

fn confirm_action(id: &str, label: &str, risk: ActionRisk) -> ActionDescriptor {
    ActionDescriptor {
        id: ActionId::new(id),
        label: label.into(),
        risk,
        confirmation: true,
    }
}

fn invalid_input(message: &str) -> ActionOutcome {
    ActionOutcome::Failed {
        kind: FailureKind::InvalidInput {
            field: "renewal".into(),
            message: message.into(),
        },
    }
}

fn unavailable(reason: String) -> ActionOutcome {
    ActionOutcome::Failed {
        kind: FailureKind::Unavailable {
            reason,
            retryable: true,
        },
    }
}

fn confirmation_required(action: &str) -> ActionOutcome {
    ActionOutcome::Failed {
        kind: FailureKind::SecurityDenied {
            reason: format!("confirmation required to {action}"),
        },
    }
}

fn repo_outcome(error: RenewalsRepoError) -> ActionOutcome {
    ActionOutcome::Failed {
        kind: match error {
            RenewalsRepoError::NotFound => FailureKind::NotFound {
                entity: "renewal".into(),
            },
            RenewalsRepoError::Conflict => FailureKind::Conflict {
                reason: "renewal changed; search again before acting".into(),
            },
            RenewalsRepoError::Capacity => FailureKind::Conflict {
                reason: format!(
                    "renewals capacity reached ({}); existing entries can still be updated",
                    luma_application::MAX_RENEWALS
                ),
            },
            RenewalsRepoError::Store(reason) => FailureKind::Unavailable {
                reason,
                retryable: true,
            },
        },
    }
}

async fn send_one(sink: &SearchSink, item: SearchItemDto) {
    let _ = sink
        .send(Event::ResultsChunk {
            request_id: String::new(),
            sequence: 1,
            upserts: vec![item],
            removed_ids: vec![],
        })
        .await;
}

async fn send_status(sink: &SearchSink, id: &str, title: &str, subtitle: &str, kind: &str) {
    send_one(
        sink,
        SearchItemDto {
            id: id.into(),
            module_id: MODULE_ID.into(),
            title: title.into(),
            subtitle: Some(subtitle.into()),
            kind: kind.into(),
            score: 0.0,
            primary_action_id: "noop".into(),
            primary_action_label: "OK".into(),
            ..Default::default()
        },
    )
    .await;
}

async fn send_repo_error(sink: &SearchSink, error: RenewalsRepoError) {
    let (title, subtitle, kind) = match error {
        RenewalsRepoError::Capacity => (
            "Renewals capacity reached",
            "The 1000-row limit permits updates but not new entries".into(),
            "capacity",
        ),
        other => (
            "Renewals store is unavailable",
            other.to_string(),
            "unavailable",
        ),
    };
    send_status(sink, "renew:unavailable", title, &subtitle, kind).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_application::{FixedClock, MemoryRenewalsRepository};
    use luma_domain::ResultId;

    fn clock() -> Arc<FixedClock> {
        Arc::new(FixedClock::new("2026-01-15", "2026-01-15T12:00:00.000Z"))
    }

    fn stored(
        repository: &MemoryRenewalsRepository,
        name: &str,
        due: &str,
        cadence: &str,
    ) -> RenewalEntry {
        repository
            .insert(&NewRenewal {
                name: name.into(),
                category: "software".into(),
                amount_minor: Some(999),
                currency: Some("USD".into()),
                cadence_kind: cadence.into(),
                cadence_value: None,
                anchor_month: Some(1),
                anchor_day: Some(31),
                next_due_date: due.into(),
                auto_renew: cadence != "once",
                status: "active".into(),
                url: None,
                note: None,
                now: "v1".into(),
            })
            .unwrap()
    }

    fn item(entry: &RenewalEntry, action: &str, risk: ActionRisk) -> SearchItem {
        let confirmation = risk != ActionRisk::Safe;
        SearchItem {
            id: ResultId::new(format!("renew:{}", entry.id)),
            module_id: ModuleId::new(MODULE_ID),
            title: entry.name.clone(),
            subtitle: None,
            kind: "renewal".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new(action),
                label: action.into(),
                risk,
                confirmation,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: Some(identity_payload(entry)),
        }
    }

    #[test]
    fn currency_precision_is_explicit_and_integer_only() {
        assert_eq!(
            parse_amount_currency("12.34 USD").unwrap(),
            (Some(1234), Some("USD".into()))
        );
        assert_eq!(
            parse_amount_currency("12 JPY").unwrap(),
            (Some(12), Some("JPY".into()))
        );
        assert_eq!(
            parse_amount_currency("1.234 KWD").unwrap(),
            (Some(1234), Some("KWD".into()))
        );
        assert!(parse_amount_currency("12.3 JPY").is_err());
        assert!(parse_amount_currency("12.345 USD").is_err());
        assert!(parse_amount_currency("1.0 XYZ").is_err());
        assert!(parse_amount_currency("1e3 USD").is_err());
    }

    #[test]
    fn recurrence_retains_month_end_and_leap_anchors() {
        let repository = MemoryRenewalsRepository::default();
        let january = stored(&repository, "Monthly", "2024-01-31", "monthly");
        assert_eq!(advance_after_paid(&january).unwrap().0, "2024-02-29");
        let mut february = january;
        february.next_due_date = "2024-02-29".into();
        assert_eq!(advance_after_paid(&february).unwrap().0, "2024-03-31");

        let mut yearly = february;
        yearly.cadence_kind = "yearly".into();
        yearly.anchor_month = Some(2);
        yearly.anchor_day = Some(29);
        assert_eq!(advance_after_paid(&yearly).unwrap().0, "2025-02-28");
        yearly.next_due_date = "2027-02-28".into();
        assert_eq!(advance_after_paid(&yearly).unwrap().0, "2028-02-29");
    }

    #[test]
    fn quarterly_custom_and_once_recurrences_are_deterministic() {
        let repository = MemoryRenewalsRepository::default();
        let mut entry = stored(&repository, "Quarterly", "2024-11-30", "quarterly");
        entry.anchor_day = Some(31);
        assert_eq!(advance_after_paid(&entry).unwrap().0, "2025-02-28");
        entry.cadence_kind = "custom_days".into();
        entry.cadence_value = Some(10);
        assert_eq!(advance_after_paid(&entry).unwrap().0, "2024-12-10");
        entry.cadence_kind = "once".into();
        assert_eq!(
            advance_after_paid(&entry).unwrap(),
            ("2024-11-30".into(), "completed".into())
        );
    }

    #[tokio::test]
    async fn paid_advances_from_scheduled_date_and_is_idempotent() {
        let repository = Arc::new(MemoryRenewalsRepository::default());
        let entry = stored(&repository, "Cloud", "2024-01-31", "monthly");
        let module = RenewalsModule::with_deps(repository.clone(), clock());
        let paid = safe_action("paid", "Mark paid");
        let result = item(&entry, "paid", ActionRisk::Safe);
        let outcome = module
            .perform(
                ActionRequest {
                    result: result.clone(),
                    action: paid.clone(),
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(outcome, ActionOutcome::Success { .. }));
        assert_eq!(
            repository.get(entry.id).unwrap().unwrap().next_due_date,
            "2024-02-29"
        );
        assert!(repository
            .get(entry.id)
            .unwrap()
            .unwrap()
            .updated_at
            .starts_with("2026-01-15T12:00:00.000Z#"));
        let duplicate = module
            .perform(
                ActionRequest {
                    result,
                    action: paid,
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(
            duplicate,
            ActionOutcome::Failed {
                kind: FailureKind::Conflict { .. }
            }
        ));
    }

    #[tokio::test]
    async fn cancel_and_delete_require_confirmation_and_fresh_identity() {
        let repository = Arc::new(MemoryRenewalsRepository::default());
        let entry = stored(&repository, "Cloud", "2026-01-31", "monthly");
        let module = RenewalsModule::with_deps(repository.clone(), clock());
        let cancel_action = confirm_action("cancel", "Cancel", ActionRisk::Confirm);
        let denied = module
            .perform(
                ActionRequest {
                    result: item(&entry, "cancel", ActionRisk::Confirm),
                    action: cancel_action.clone(),
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(
            denied,
            ActionOutcome::Failed {
                kind: FailureKind::SecurityDenied { .. }
            }
        ));
        assert_eq!(repository.get(entry.id).unwrap().unwrap().status, "active");

        let mut changed = repository.get(entry.id).unwrap().unwrap();
        changed.note = Some("changed".into());
        changed.updated_at = "v2".into();
        repository.update(&changed, "v1").unwrap();
        let stale = module
            .perform(
                ActionRequest {
                    result: item(&entry, "delete", ActionRisk::Destructive),
                    action: confirm_action("delete", "Delete", ActionRisk::Destructive),
                    confirmation: true,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(
            stale,
            ActionOutcome::Failed {
                kind: FailureKind::Conflict { .. }
            }
        ));
        assert!(repository.get(entry.id).unwrap().is_some());
    }

    #[tokio::test]
    async fn completed_once_renewal_cannot_be_cancelled() {
        let repository = Arc::new(MemoryRenewalsRepository::default());
        let entry = stored(&repository, "One time", "2026-01-31", "once");
        let module = RenewalsModule::with_deps(repository.clone(), clock());
        let paid = module
            .perform(
                ActionRequest {
                    result: item(&entry, "paid", ActionRisk::Safe),
                    action: safe_action("paid", "Paid"),
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(paid, ActionOutcome::Success { .. }));

        let completed = repository.get(entry.id).unwrap().unwrap();
        let cancelled = module
            .perform(
                ActionRequest {
                    result: item(&completed, "cancel", ActionRisk::Confirm),
                    action: confirm_action("cancel", "Cancel", ActionRisk::Confirm),
                    confirmation: true,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(
            cancelled,
            ActionOutcome::Failed {
                kind: FailureKind::Conflict { .. }
            }
        ));
        assert_eq!(
            repository.get(entry.id).unwrap().unwrap().status,
            "completed"
        );
    }

    #[tokio::test]
    async fn cancellation_prevents_mutation() {
        let repository = Arc::new(MemoryRenewalsRepository::default());
        let entry = stored(&repository, "Cloud", "2026-01-31", "monthly");
        let module = RenewalsModule::with_deps(repository.clone(), clock());
        let cancel = CancellationToken::new();
        cancel.cancel();
        let outcome = module
            .perform(
                ActionRequest {
                    result: item(&entry, "paid", ActionRisk::Safe),
                    action: safe_action("paid", "Paid"),
                    confirmation: false,
                },
                cancel,
            )
            .await;
        assert_eq!(outcome, ActionOutcome::Cancelled);
        assert_eq!(
            repository.get(entry.id).unwrap().unwrap().next_due_date,
            "2026-01-31"
        );
    }

    #[tokio::test]
    async fn invalid_add_is_visible_and_empty_state_is_explicit() {
        let module =
            RenewalsModule::with_deps(Arc::new(MemoryRenewalsRepository::default()), clock());
        let (sink, mut receiver) = tokio::sync::mpsc::channel(2);
        module
            .search(
                Query::parse_with_prefixes_strict(
                    "/renew add Bad | 2026-02-30 | 1 USD | monthly",
                    20,
                    |prefix| matches!(prefix, "renew" | "renewals"),
                ),
                sink,
                CancellationToken::new(),
            )
            .await;
        let Event::ResultsChunk { upserts, .. } = receiver.recv().await.unwrap() else {
            panic!("expected results");
        };
        assert_eq!(upserts[0].kind, "command_error");

        let (sink, mut receiver) = tokio::sync::mpsc::channel(2);
        module
            .search(
                Query::parse_with_prefixes_strict("/renew ", 20, |prefix| {
                    matches!(prefix, "renew" | "renewals")
                }),
                sink,
                CancellationToken::new(),
            )
            .await;
        let Event::ResultsChunk { upserts, .. } = receiver.recv().await.unwrap() else {
            panic!("expected results");
        };
        assert_eq!(upserts[0].id, "renew:empty");
    }
}
