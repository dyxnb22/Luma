use crate::effect::Effect;
use crate::view_model::{AppState, FocusZone, PendingAction, Route, StatusTone};
use luma_protocol::ActionDescriptorDto;

pub(super) fn wordbook_review_queue_from_prompt(prompt: &str) -> Option<String> {
    let lower = super::explicit_command_prompt(prompt)?.to_ascii_lowercase();
    if lower == "wb review" || lower == "wb review due" {
        return Some("due".into());
    }
    if lower == "wb review new" {
        return Some("new".into());
    }
    if lower == "wb review wrong" {
        return Some("wrong".into());
    }
    None
}

pub(super) fn wordbook_review_queue_from_item(item: &luma_domain::SearchItem) -> Option<String> {
    if item.primary_action.id.as_str() == "start_review" {
        return item
            .action_payload
            .as_ref()
            .and_then(|p| p.get("queue"))
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .or_else(|| {
                item.id
                    .as_str()
                    .strip_prefix("wb:review:")
                    .map(str::to_string)
            });
    }
    item.id
        .as_str()
        .strip_prefix("wb:review:")
        .map(str::to_string)
}

pub(super) fn begin_wordbook_review(state: &mut AppState, queue: String) -> Vec<Effect> {
    state.overlay_restore_prompt = Some(state.prompt.clone());
    state.clear_prompt();
    state.search_debounce_deadline = None;
    state.route = Route::WordbookReview;
    state.wordbook_review = None;
    state
        .status
        .set(format!("loading review ({queue})…"), StatusTone::Progress);
    vec![Effect::LoadWordbookReview { queue }]
}

pub(super) fn wordbook_reveal(state: &mut AppState) -> Vec<Effect> {
    if state.route != Route::WordbookReview {
        return vec![Effect::None];
    }
    if let Some(review) = state.wordbook_review.as_mut() {
        if !review.finished {
            review.revealed = true;
        }
    }
    vec![Effect::None]
}

pub(super) fn wordbook_grade(state: &mut AppState, action_id: String) -> Vec<Effect> {
    if state.route != Route::WordbookReview {
        return vec![Effect::None];
    }
    let Some(review) = state.wordbook_review.as_ref() else {
        return vec![Effect::None];
    };
    if review.finished || state.active_operation.is_some() {
        return vec![Effect::None];
    }
    let Some(word_id) = review.words.get(review.index).map(|w| w.id) else {
        return vec![Effect::None];
    };
    let revealed = review.revealed;
    if action_id == "skip" {
        if let Some(review) = state.wordbook_review.as_mut() {
            review.stats.session_skipped += 1;
            review.revealed = false;
            review.index += 1;
            if review.index >= review.words.len() {
                review.finished = true;
            }
        }
        if state.wordbook_review.as_ref().is_some_and(|r| r.finished) {
            state
                .status
                .set("review done · skipped", StatusTone::Success);
        }
        return vec![Effect::None];
    }
    if !revealed {
        return vec![Effect::None];
    }
    let result_id = format!("wb:{word_id}");
    let mastered = action_id == "mastered";
    let action = ActionDescriptorDto {
        id: action_id.clone(),
        label: action_id.clone(),
        risk: if mastered {
            luma_domain::ActionRisk::Confirm
        } else {
            luma_domain::ActionRisk::Safe
        },
        confirmation: mastered,
    };
    if let Some(review) = state.wordbook_review.as_mut() {
        review.pending_grade = Some(action_id.clone());
    }
    if mastered {
        state.pending_action = Some(PendingAction { result_id, action });
        state.route = Route::ConfirmAction;
        state
            .status
            .set("confirm mastered? Enter=yes Esc=no", StatusTone::Warning);
        return vec![Effect::None];
    }
    super::execute_action(state, result_id, action, false)
}

pub(super) fn exit_wordbook_review(state: &mut AppState) -> Vec<Effect> {
    state.wordbook_review = None;
    state.route = Route::Search;
    if let Some(prompt) = state.overlay_restore_prompt.take() {
        state.prompt = prompt;
        state.prompt_cursor = state.prompt_char_len();
    }
    state.focus = FocusZone::Prompt;
    state.status.set("review ended", StatusTone::Neutral);
    vec![Effect::None]
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use crate::msg::Msg;
    use crate::view_model::{AppState, Route, StatusTone};
    use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ModuleId, ResultId, SearchItem};
    use luma_protocol::{ActionOutcomeDto, Event};

    fn sample_wordbook_review(words: Vec<(i64, &str)>) -> AppState {
        let mut state = AppState::default();
        state.route = Route::WordbookReview;
        state.wordbook_review = Some(crate::view_model::WordbookReviewState {
            words: words
                .into_iter()
                .map(|(id, term)| crate::view_model::WordbookReviewWord {
                    id,
                    term: term.into(),
                    phonetic: String::new(),
                    meaning: format!("meaning-{term}"),
                    example: String::new(),
                })
                .collect(),
            index: 0,
            revealed: false,
            stats: crate::view_model::WordbookReviewStats {
                queue: "due".into(),
                due: 2,
                goal: 20,
                reviewed_today: 7,
                remaining_goal: 13,
                ..Default::default()
            },
            finished: false,
            pending_grade: None,
        });
        state
    }

    #[test]
    fn wordbook_review_starts_from_prompt() {
        let mut state = AppState::default();
        state.prompt = "/wb review due".into();
        state.prompt_cursor = state.prompt_char_len();
        let effects = super::super::update(&mut state, Msg::Submit);
        assert_eq!(state.route, Route::WordbookReview);
        assert!(state.prompt.is_empty());
        assert!(effects.iter().any(|e| matches!(
            e,
            Effect::LoadWordbookReview { queue } if queue == "due"
        )));
    }

    #[test]
    fn wordbook_grade_blocks_before_reveal() {
        let mut state = sample_wordbook_review(vec![(1, "alpha")]);
        let effects = super::super::update(
            &mut state,
            Msg::WordbookGrade {
                action_id: "known".into(),
            },
        );
        assert_eq!(effects, vec![Effect::None]);
        assert_eq!(state.wordbook_review.as_ref().unwrap().index, 0);
    }

    #[test]
    fn wordbook_reveal_then_known_advances() {
        let mut state = sample_wordbook_review(vec![(1, "alpha"), (2, "beta")]);
        let _ = super::super::update(&mut state, Msg::WordbookReveal);
        assert!(state.wordbook_review.as_ref().unwrap().revealed);
        let effects = super::super::update(
            &mut state,
            Msg::WordbookGrade {
                action_id: "known".into(),
            },
        );
        assert!(effects.iter().any(|e| matches!(
            e,
            Effect::ExecuteAction { result_id, action_id, .. }
            if result_id == "wb:1" && action_id == "known"
        )));
        state.active_operation = Some("op-1".into());
        let _ = state.apply_engine_event(Event::ActionFinished {
            operation_id: "op-1".into(),
            outcome: ActionOutcomeDto::Success {
                message: Some("ok".into()),
            },
        });
        let review = state.wordbook_review.as_ref().unwrap();
        assert_eq!(review.index, 1);
        assert!(!review.revealed);
        assert_eq!(review.stats.session_known, 1);
        assert_eq!(review.stats.reviewed_today, 7);
        let _ = state.apply_engine_event(Event::WordbookReviewStatsUpdated {
            stats: luma_protocol::WordbookStatsDto {
                due: 2,
                new_count: 0,
                wrong: 0,
                goal: 20,
                reviewed_today: 8,
                remaining_goal: 12,
            },
        });
        assert_eq!(
            state.wordbook_review.as_ref().unwrap().stats.reviewed_today,
            8
        );
    }

    #[test]
    fn wordbook_skip_advances_without_action() {
        let mut state = sample_wordbook_review(vec![(1, "alpha"), (2, "beta")]);
        let effects = super::super::update(
            &mut state,
            Msg::WordbookGrade {
                action_id: "skip".into(),
            },
        );
        assert_eq!(effects, vec![Effect::None]);
        let review = state.wordbook_review.as_ref().unwrap();
        assert_eq!(review.index, 1);
        assert_eq!(review.stats.session_skipped, 1);
    }

    #[test]
    fn wordbook_skip_completion_sets_done_status() {
        let mut state = sample_wordbook_review(vec![(1, "alpha")]);
        let _ = super::super::update(
            &mut state,
            Msg::WordbookGrade {
                action_id: "skip".into(),
            },
        );
        assert!(state.wordbook_review.as_ref().unwrap().finished);
        assert_eq!(state.status.tone, StatusTone::Success);
        assert!(state.status.text.starts_with("review done"));
    }

    #[test]
    fn wordbook_mastered_requires_confirm() {
        let mut state = sample_wordbook_review(vec![(1, "alpha")]);
        let _ = super::super::update(&mut state, Msg::WordbookReveal);
        let effects = super::super::update(
            &mut state,
            Msg::WordbookGrade {
                action_id: "mastered".into(),
            },
        );
        assert_eq!(effects, vec![Effect::None]);
        assert_eq!(state.route, Route::ConfirmAction);
        assert!(state.pending_action.is_some());
        let confirm_effects = super::super::update(&mut state, Msg::Submit);
        assert!(confirm_effects.iter().any(|e| matches!(
            e,
            Effect::ExecuteAction { result_id, action_id, confirmation: true, .. }
            if result_id == "wb:1" && action_id == "mastered"
        )));
        assert_eq!(state.route, Route::WordbookReview);
    }

    #[test]
    fn wordbook_esc_exits_review() {
        let mut state = sample_wordbook_review(vec![(1, "alpha")]);
        state.overlay_restore_prompt = Some("/wb review".into());
        let _ = super::super::update(&mut state, Msg::Cancel);
        assert_eq!(state.route, Route::Search);
        assert!(state.wordbook_review.is_none());
        assert_eq!(state.prompt, "/wb review");
    }

    #[test]
    fn wordbook_esc_cancels_active_grade_before_exiting() {
        let mut state = sample_wordbook_review(vec![(1, "alpha")]);
        state.active_operation = Some("op-1".into());
        state.wordbook_review.as_mut().unwrap().pending_grade = Some("known".into());
        let effects = super::super::update(&mut state, Msg::Cancel);
        assert_eq!(state.route, Route::WordbookReview);
        assert_eq!(
            effects,
            vec![Effect::CancelOperation {
                operation_id: "op-1".into()
            }]
        );

        let applied = state.apply_engine_event(Event::ActionFinished {
            operation_id: "op-1".into(),
            outcome: ActionOutcomeDto::Cancelled,
        });
        assert!(applied);
        assert!(state
            .wordbook_review
            .as_ref()
            .unwrap()
            .pending_grade
            .is_none());
        assert_eq!(state.route, Route::WordbookReview);
    }

    #[test]
    fn wordbook_review_starts_from_search_result() {
        let mut state = AppState::default();
        state.prompt = "/wb review".into();
        state.prompt_cursor = state.prompt_char_len();
        state.results.items.push(SearchItem {
            id: ResultId::new("wb:review:due"),
            module_id: ModuleId::new("luma.wordbook"),
            title: "Start review (due)".into(),
            subtitle: None,
            kind: "command".into(),
            score: 100.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("start_review"),
                label: "Start review".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: Some(serde_json::json!({ "queue": "due" })),
        });
        state.results.selected_id = Some("wb:review:due".into());
        let effects = super::super::update(&mut state, Msg::Submit);
        assert_eq!(state.route, Route::WordbookReview);
        assert!(effects.iter().any(|e| matches!(
            e,
            Effect::LoadWordbookReview { queue } if queue == "due"
        )));
    }
}
