//! Tauri command surface for "the user just told us what they did with the
//! task". Loads the cat + recent task events, runs `cat_state::apply_task_outcome`,
//! persists the result, and returns the updated cat plus an `OutcomeEffect`
//! the frontend uses to decide whether to regenerate the portrait.

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::cat_state::{self, OutcomeEffect, OutcomePayload};
use crate::model::{Cat, SkillId, TaskEvent};
use crate::store;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyTaskOutcomeResponse {
    pub cat: Cat,
    pub effect: OutcomeEffect,
}

#[tauri::command]
pub fn apply_task_outcome(
    app: AppHandle,
    payload: OutcomePayload,
    last_event: Option<TaskEvent>,
) -> Result<ApplyTaskOutcomeResponse, String> {
    let mut history = store::read_task_events(&app).map_err(|e| e.to_string())?;
    if let Some(event) = last_event {
        // The frontend records the event in parallel with this call —
        // we may run before or after that write. Either way, make sure the
        // streak math sees this completion.
        if !history.iter().any(|e| e.id == event.id) {
            history.push(event);
        }
    }
    let Some(mut cat) = store::read_cat(&app).map_err(|e| e.to_string())? else {
        return Err("no cat found — finish onboarding first".into());
    };
    let effect =
        cat_state::apply_task_outcome(&mut cat, payload.category, payload.outcome, &history);
    store::write_cat(&app, &cat).map_err(|e| e.to_string())?;
    Ok(ApplyTaskOutcomeResponse { cat, effect })
}

/// Helper for the dashboard to expose what the cat has earned.
#[tauri::command]
pub fn list_cat_skills(app: AppHandle) -> Result<Vec<SkillId>, String> {
    Ok(store::read_cat(&app)
        .map_err(|e| e.to_string())?
        .map(|c| c.skills)
        .unwrap_or_default())
}
