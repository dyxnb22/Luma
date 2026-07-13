use crate::effect::Effect;
use crate::msg::Msg;
use crate::view_model::{AppState, Route};

/// Pure synchronous reducer. Must not perform I/O.
pub fn update(state: &mut AppState, msg: Msg) -> Vec<Effect> {
    state.dirty = true;
    match msg {
        Msg::KeyChar(c) => {
            if state.route != Route::Search {
                state.route = Route::Search;
            }
            state.prompt.push(c);
            begin_search(state)
        }
        Msg::Backspace => {
            state.prompt.pop();
            begin_search(state)
        }
        Msg::Submit => {
            if state.prompt.trim() == ":doctor" {
                state.prompt.clear();
                state.route = Route::Doctor;
                state.status.text = "doctor".into();
                return vec![Effect::RunDoctor];
            }
            if state.prompt.trim() == ":help" || state.prompt.trim() == "?" {
                state.prompt.clear();
                state.route = Route::Help;
                state.status.text = "help".into();
                return vec![Effect::None];
            }
            state.status.text = "running primary action".into();
            vec![Effect::None]
        }
        Msg::SelectNext => {
            state.results.select_next();
            vec![Effect::None]
        }
        Msg::SelectPrev => {
            state.results.select_prev();
            vec![Effect::None]
        }
        Msg::OpenHelp => {
            state.route = Route::Help;
            state.status.text = "help".into();
            vec![Effect::None]
        }
        Msg::OpenDoctor => {
            state.route = Route::Doctor;
            state.status.text = "doctor".into();
            vec![Effect::RunDoctor]
        }
        Msg::Quit => {
            state.should_quit = true;
            cancel_active(state)
        }
        Msg::Cancel => {
            if state.route != Route::Search {
                state.route = Route::Search;
                return vec![Effect::None];
            }
            if state.active_request.is_some() {
                let effects = cancel_active(state);
                state.status.text = "cancelled".into();
                effects
            } else if !state.prompt.is_empty() {
                state.prompt.clear();
                state.results.items.clear();
                state.results.selected_id = None;
                state.active_request = None;
                vec![Effect::None]
            } else {
                state.should_quit = true;
                vec![Effect::None]
            }
        }
        Msg::Redraw | Msg::Resize | Msg::Tick => vec![Effect::None],
        Msg::Engine(event) => {
            let _ = state.apply_engine_event(event);
            vec![Effect::None]
        }
    }
}

fn begin_search(state: &mut AppState) -> Vec<Effect> {
    let mut effects = cancel_active(state);
    if state.prompt.is_empty() {
        state.results.items.clear();
        state.results.selected_id = None;
        state.status.text = "ready".into();
        return effects;
    }
    let request_id = next_request_id(state);
    state.active_request = Some(request_id.clone());
    state.request_seq_seen = 0;
    state.results.items.clear();
    state.results.selected_id = None;
    effects.push(Effect::Search {
        request_id,
        query: state.prompt.clone(),
    });
    effects
}

fn cancel_active(state: &mut AppState) -> Vec<Effect> {
    if let Some(request_id) = state.active_request.take() {
        vec![Effect::CancelSearch { request_id }]
    } else {
        Vec::new()
    }
}

fn next_request_id(state: &mut AppState) -> String {
    state.search_generation = state.search_generation.saturating_add(1);
    format!("req-{}", state.search_generation)
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use luma_protocol::{Event, SearchItemDto};

    #[test]
    fn typing_starts_new_search_and_cancels_old() {
        let mut state = AppState::default();
        let effects = update(&mut state, Msg::KeyChar('a'));
        assert!(matches!(effects.last(), Some(Effect::Search { .. })));
        let first = state.active_request.clone().unwrap();

        let effects = update(&mut state, Msg::KeyChar('p'));
        assert!(effects
            .iter()
            .any(|e| matches!(e, Effect::CancelSearch { request_id } if request_id == &first)));
        assert!(effects.iter().any(|e| matches!(e, Effect::Search { .. })));
    }

    #[test]
    fn late_chunk_from_old_request_is_ignored() {
        let mut state = AppState::default();
        let _ = update(&mut state, Msg::KeyChar('a'));
        let active = state.active_request.clone().unwrap();

        // Simulate superseding search
        let _ = update(&mut state, Msg::KeyChar('b'));
        let new_active = state.active_request.clone().unwrap();
        assert_ne!(active, new_active);

        let applied = state.apply_engine_event(Event::ResultsChunk {
            request_id: active,
            sequence: 1,
            upserts: vec![SearchItemDto {
                id: "old".into(),
                module_id: "mock".into(),
                title: "old".into(),
                subtitle: None,
                kind: "mock".into(),
                score: 1.0,
                primary_action_id: "open".into(),
                primary_action_label: "Open".into(),
            }],
            removed_ids: vec![],
        });
        assert!(!applied);
        assert!(state.results.items.is_empty());
    }

    #[test]
    fn chunk_for_active_request_updates_results() {
        let mut state = AppState::default();
        let _ = update(&mut state, Msg::KeyChar('x'));
        let active = state.active_request.clone().unwrap();
        let applied = state.apply_engine_event(Event::ResultsChunk {
            request_id: active,
            sequence: 1,
            upserts: vec![SearchItemDto {
                id: "1".into(),
                module_id: "mock".into(),
                title: "Alpha".into(),
                subtitle: None,
                kind: "mock".into(),
                score: 10.0,
                primary_action_id: "open".into(),
                primary_action_label: "Open".into(),
            }],
            removed_ids: vec![],
        });
        assert!(applied);
        assert_eq!(state.results.items.len(), 1);
        assert_eq!(state.results.selected_id.as_deref(), Some("1"));
    }

    #[test]
    fn doctor_meta_emits_run_doctor_not_primary() {
        use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ModuleId, ResultId, SearchItem};
        let mut state = AppState::default();
        // Seed a selected result that must NOT be actioned by :doctor.
        state.results.items.push(SearchItem {
            id: ResultId::new("danger"),
            module_id: ModuleId::new("mock"),
            title: "Danger".into(),
            subtitle: None,
            kind: "mock".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("open"),
                label: "Open".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
        });
        state.results.selected_id = Some("danger".into());
        state.prompt = ":doctor".into();
        let effects = update(&mut state, Msg::Submit);
        assert_eq!(effects, vec![Effect::RunDoctor]);
        assert_eq!(state.route, Route::Doctor);
        assert_eq!(state.status.text, "doctor");
        assert_ne!(state.status.text, "running primary action");
    }

    #[test]
    fn help_meta_does_not_run_doctor_or_primary_status() {
        let mut state = AppState::default();
        state.prompt = ":help".into();
        let effects = update(&mut state, Msg::Submit);
        assert_eq!(effects, vec![Effect::None]);
        assert_eq!(state.route, Route::Help);
        assert_eq!(state.status.text, "help");
    }

    #[test]
    fn cancel_quits_from_empty_search() {
        let mut state = AppState::default();
        let _ = update(&mut state, Msg::Cancel);
        assert!(state.should_quit);
    }
}
