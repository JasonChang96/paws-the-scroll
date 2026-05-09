//! Global shortcuts for deterministic demo control. Pattern mirrors
//! tambourine-voice: one `on_shortcut` registration per binding, each with
//! its own closure that dispatches into the relevant subsystem.
//!
//! Registered shortcuts:
//!  - `Cmd+Ctrl+Opt+P` ("Paws"): summon the cat (fire interruption now).
//!  - `Cmd+Ctrl+Opt+E` ("Evolve"): bump the cat's `independence_level` by
//!    one tier and regenerate the portrait — useful for showing visual
//!    evolution on stage without waiting through task completions.
//!
//! With demo mode on in Settings, the activity scheduler stops auto-firing
//! interruptions, so these shortcuts become the only way to drive the cat.

use crate::model::IndependenceTier;
use tauri::{AppHandle, Runtime};

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub fn register<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    use tauri_plugin_global_shortcut::{
        Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
    };

    let manager = app.global_shortcut();
    let summon = Shortcut::new(
        Some(Modifiers::SUPER | Modifiers::CONTROL | Modifiers::ALT),
        Code::KeyP,
    );
    let evolve = Shortcut::new(
        Some(Modifiers::SUPER | Modifiers::CONTROL | Modifiers::ALT),
        Code::KeyE,
    );

    if let Err(error) = manager.on_shortcut(summon, |handle, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            if let Err(error) = crate::activity::request_interruption(handle.clone()) {
                log::warn!("[demo-trigger] summon failed: {error}");
            }
        }
    }) {
        log::warn!("[demo-trigger] failed to register summon shortcut: {error}");
    } else {
        log::info!("[demo-trigger] registered Cmd+Ctrl+Opt+P (summon)");
    }

    if let Err(error) = manager.on_shortcut(evolve, |handle, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            let app_handle = handle.clone();
            // Spawn into Tauri's async runtime so the shortcut callback
            // (which must return immediately) doesn't block on the OpenAI
            // edit call. The handler bumps state, persists, then awaits the
            // portrait regen — `cat-updated` already fires from
            // store::write_cat so the overlay picks up the new state path
            // synchronously, and swaps to the new image when regen lands.
            tauri::async_runtime::spawn(async move {
                force_evolve_cat(&app_handle).await;
            });
        }
    }) {
        log::warn!("[demo-trigger] failed to register evolve shortcut: {error}");
    } else {
        log::info!("[demo-trigger] registered Cmd+Ctrl+Opt+E (evolve)");
    }

    Ok(())
}

#[cfg(any(target_os = "android", target_os = "ios"))]
pub fn register<R: Runtime>(_app: &AppHandle<R>) -> tauri::Result<()> {
    Ok(())
}

/// Bump the cat's `independence_level` by 0.25 (one tier), append a story
/// event, persist, then regenerate the portrait via the edit endpoint so
/// the visual change is actually visible. Async because the regen call
/// streams from `OpenAI`.
async fn force_evolve_cat<R: Runtime>(app: &AppHandle<R>) {
    let Ok(Some(mut cat)) = crate::store::read_cat(app) else {
        log::warn!("[demo-trigger] evolve: no cat to evolve");
        return;
    };
    let previous = cat.independence_level;
    cat.independence_level = (cat.independence_level + 0.25).min(1.0);
    cat.story_events.push(crate::model::StoryEvent {
        id: format!("se_{}", uuid::Uuid::new_v4()),
        at: chrono::Utc::now(),
        text: format!(
            "{} suddenly seems a little more sure of themselves.",
            cat.name
        ),
    });
    if cat.story_events.len() > 50 {
        let excess = cat.story_events.len() - 50;
        cat.story_events.drain(0..excess);
    }
    if let Err(error) = crate::store::write_cat(app, &cat) {
        log::warn!("[demo-trigger] evolve: failed to persist cat: {error}");
        return;
    }
    log::info!(
        "[demo-trigger] evolved cat: independence {previous:.2} -> {:.2}; regenerating portrait",
        cat.independence_level
    );

    // Build a portrait request from the cat's now-current state and run the
    // edit pipeline. Same code path the task-completion flow uses; partials
    // stream over `cat-portrait-progress` to the overlay.
    let request = crate::openai::PortraitRequest {
        cat_id: cat.id.clone(),
        cat_type: cat.cat_type,
        mood: cat.mood,
        independence_tier: IndependenceTier::from_level(cat.independence_level),
        accessory_set_hash: "v1".into(),
        skills: cat.skills.clone(),
    };
    match crate::openai::generate_portrait(app, &request).await {
        Ok(response) => {
            cat.portrait_path = Some(response.path);
            cat.portrait_is_base = false;
            if let Err(error) = crate::store::write_cat(app, &cat) {
                log::warn!("[demo-trigger] evolve: portrait_path write failed: {error}");
            }
        }
        Err(error) => log::warn!("[demo-trigger] evolve: portrait regen failed: {error}"),
    }
}
