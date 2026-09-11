use crate::effect::Effect;
use crate::view_model::AppState;

pub(super) fn sync_prompt_viewport(state: &mut AppState) {
    let inner_w = state.terminal.width.saturating_sub(2) as usize;
    state.ensure_prompt_visible(inner_w.max(20));
}

pub(super) fn preview_effect(state: &mut AppState) -> Vec<Effect> {
    let Some(result_id) = state.search.results.selected_id.clone() else {
        state.preview.pending_id = None;
        return vec![Effect::None];
    };
    if state.preview.result_id.as_deref() == Some(result_id.as_str())
        && state.preview.body.is_some()
    {
        return vec![Effect::None];
    }
    if state.preview.pending_id.is_some()
        && state.preview.result_id.as_deref() == Some(result_id.as_str())
    {
        return vec![Effect::None];
    }
    state.preview.generation = state.preview.generation.saturating_add(1);
    let preview_id = state.preview.generation;
    state.preview.pending_id = Some(preview_id);
    state.preview.result_id = Some(result_id.clone());
    state.preview.body = None;
    vec![Effect::LoadPreview {
        result_id,
        preview_id,
    }]
}
