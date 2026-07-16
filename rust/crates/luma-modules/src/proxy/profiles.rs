use super::redact::{opaque_component, redact_label};
use super::MODULE_ID;
use luma_application::{ProfileSource, ProfileStoreError, ProfileSummary};
use luma_domain::ActionRisk;
use luma_protocol::SearchItemDto;

pub(super) fn profile_item(profile: ProfileSummary) -> SearchItemDto {
    let updated = profile
        .updated_at
        .map(|v| v.to_string())
        .unwrap_or_else(|| "unknown".into());
    let primary = if profile.owned_by_luma {
        "use_profile"
    } else {
        "noop"
    };
    let details = if !profile.metadata_available {
        format!(
            "metadata unavailable · updated {} · {}{}",
            updated,
            profile.source.label(),
            if profile.current { " · current" } else { "" }
        )
    } else {
        format!(
            "{} nodes · {} groups · {} rules · updated {} · {}{}",
            profile.node_count,
            profile.group_count,
            profile.rule_count,
            updated,
            profile.source.label(),
            if profile.current { " · current" } else { "" }
        )
    };
    let owned_by_luma = profile.owned_by_luma;
    let external_uid_in_name = !owned_by_luma
        && profile
            .name
            .to_lowercase()
            .contains(&profile.id.to_lowercase());
    let title = if external_uid_in_name {
        // Clash Verge's UID is an internal identifier. Do not let a user-controlled display name
        // smuggle it back into UI when this item is deliberately read-only.
        "Clash Verge Profile".into()
    } else {
        redact_label(&profile.name)
    };
    let mut secondary_actions = Vec::new();
    if owned_by_luma {
        secondary_actions.push(action_dto(
            "delete_profile",
            "Delete",
            ActionRisk::Confirm,
            true,
        ));
        if profile.source == ProfileSource::Subscription {
            secondary_actions.push(action_dto(
                "refresh_profile",
                "Refresh",
                ActionRisk::Safe,
                false,
            ));
        }
    }
    SearchItemDto {
        id: if owned_by_luma {
            format!("proxy:profile:{}", profile.id)
        } else {
            format!("proxy:profile:readonly:{}", opaque_component(&profile.id))
        },
        module_id: MODULE_ID.into(),
        title,
        subtitle: Some(details),
        kind: "profile".into(),
        score: if profile.current { 100.0 } else { 85.0 },
        primary_action_id: primary.into(),
        primary_action_label: if owned_by_luma {
            "Use".into()
        } else {
            "Read-only".into()
        },
        primary_action_risk: if owned_by_luma {
            ActionRisk::Confirm
        } else {
            ActionRisk::Safe
        },
        primary_action_confirmation: owned_by_luma,
        secondary_actions,
        action_payload: owned_by_luma.then(|| serde_json::json!({"profile_id": profile.id})),
        ..Default::default()
    }
}

pub(super) fn profile_unavailable() -> SearchItemDto {
    SearchItemDto {
        id: "proxy:profile:unavailable".into(),
        module_id: MODULE_ID.into(),
        title: "Profiles unavailable".into(),
        subtitle: Some("Luma Profile storage is not configured".into()),
        kind: "unavailable".into(),
        primary_action_id: "refresh".into(),
        primary_action_label: "Refresh".into(),
        ..Default::default()
    }
}

pub(super) fn profile_error_item(error: &ProfileStoreError) -> SearchItemDto {
    SearchItemDto {
        id: "proxy:profile:unavailable".into(),
        module_id: MODULE_ID.into(),
        title: "Profiles unavailable".into(),
        subtitle: Some(profile_error_message(error)),
        kind: "unavailable".into(),
        primary_action_id: "refresh".into(),
        primary_action_label: "Refresh".into(),
        ..Default::default()
    }
}

pub(super) fn profile_error_message(error: &ProfileStoreError) -> String {
    error.to_string()
}

pub(super) fn action_dto(
    id: &str,
    label: &str,
    risk: ActionRisk,
    confirmation: bool,
) -> luma_protocol::ActionDescriptorDto {
    luma_protocol::ActionDescriptorDto {
        id: id.into(),
        label: label.into(),
        risk,
        confirmation,
    }
}
