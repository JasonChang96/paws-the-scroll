//! Activity tracking orchestrator. Runs a 5-second tick that:
//!  - Reads `seconds_since_last_input` (`CGEventSource` on macOS).
//!  - Reads frontmost app via `NSWorkspace.frontmostApplication`.
//!  - Classifies it as social/browser/other.
//!  - Accumulates active vs idle streaks and writes minute-grained aggregates.
//!  - Decides when to fire an interruption based on a randomized window.
//!  - Credits time-away to today's aggregate after a recent interruption.
//!
//! Interruption events are emitted on the Tauri bus as
//! `"interruption-requested"`. The frontend (or demo trigger) listens and
//! drives the overlay.

#![allow(dead_code)]

#[cfg(target_os = "macos")]
mod accessibility;
mod classifier;
mod foreground;
mod idle;

use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Runtime};

use crate::cat_state;
use crate::store;

pub use classifier::AppCategory;
pub use foreground::ForegroundApp;

const TICK_INTERVAL_SECONDS: u64 = 5;
pub const INTERRUPTION_REQUESTED_EVENT: &str = "interruption-requested";

/// Where an interruption originated. The scheduler fires on the activity
/// timer; the demo trigger comes from the global hotkey.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterruptionSource {
    Scheduler,
    DemoTrigger,
}

/// Coarse time-of-day buckets the LLM uses for tone (morning tasks differ
/// from late-night ones). `match`-able so adding a bucket is a compile error
/// at every consumer.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeOfDay {
    Morning,
    Afternoon,
    Evening,
    LateNight,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterruptionPayload {
    pub source: InterruptionSource,
    pub active_app: Option<ForegroundApp>,
    pub active_app_category: AppCategory,
    pub time_of_day_label: TimeOfDay,
    /// How long the user has been continuously active (no idle gap >
    /// `idle_threshold_seconds`) leading up to this interruption. 0 for
    /// manual triggers since the user just chose to summon the cat.
    pub active_streak_seconds: u32,
    /// Today's running totals from the local aggregate. The cat uses
    /// these to say things like "you've been at this for two hours" or
    /// "this is your fourth interruption today" without us doing the
    /// math in the prompt.
    pub today_active_seconds: u32,
    pub today_social_seconds: u32,
    pub today_interruptions: u32,
    pub today_completed: u32,
    pub today_dismissed: u32,
}

#[derive(Debug)]
struct ActivityState {
    active_streak_seconds: u32,
    idle_streak_seconds: u32,
    last_interruption_at: Option<Instant>,
    next_interruption_due_in_seconds: u32,
    pending_time_away_seconds: u32,
    interruption_acknowledged: bool,
}

impl ActivityState {
    fn new(initial_window: u32) -> Self {
        Self {
            active_streak_seconds: 0,
            idle_streak_seconds: 0,
            last_interruption_at: None,
            next_interruption_due_in_seconds: initial_window,
            pending_time_away_seconds: 0,
            interruption_acknowledged: false,
        }
    }
}

fn random_window_seconds(min_s: u32, max_s: u32, social_bias: bool) -> u32 {
    use rand::RngExt;
    let mut rng = rand::rng();
    let high = if social_bias {
        max_s.midpoint(min_s).max(min_s + 1)
    } else {
        max_s.max(min_s + 1)
    };
    rng.random_range(min_s..high)
}

fn current_time_of_day() -> TimeOfDay {
    use chrono::Timelike;
    match chrono::Local::now().hour() {
        5..=11 => TimeOfDay::Morning,
        12..=16 => TimeOfDay::Afternoon,
        17..=21 => TimeOfDay::Evening,
        _ => TimeOfDay::LateNight,
    }
}

pub fn start_watcher<R: Runtime>(app: AppHandle<R>) {
    let state = Arc::new(Mutex::new(ActivityState::new(60 * 5)));

    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(TICK_INTERVAL_SECONDS));
        loop {
            ticker.tick().await;
            tick(&app, &state);
        }
    });
}

// Single coherent tick of activity state — splitting it into helpers would
// just spread the mutex lock, aggregate update, and decision logic across
// files without making the flow easier to read.
#[allow(clippy::too_many_lines)]
fn tick<R: Runtime>(app: &AppHandle<R>, state: &Mutex<ActivityState>) {
    let settings = match store::read_settings(app) {
        Ok(s) => s,
        Err(error) => {
            log::warn!("[activity] failed to read settings, skipping tick: {error}");
            return;
        }
    };
    let foreground = foreground::current_foreground_app();
    let category = foreground.as_ref().map_or(AppCategory::Other, |fg| {
        classifier::classify(fg, &settings.social_apps_extra)
    });
    let idle_seconds = idle::seconds_since_last_input();
    let is_idle = idle_seconds >= f64::from(settings.idle_threshold_seconds);

    let mut s = state.lock();

    if is_idle {
        s.active_streak_seconds = 0;
        s.idle_streak_seconds += u32::try_from(TICK_INTERVAL_SECONDS).unwrap_or(5);
        if s.interruption_acknowledged {
            s.pending_time_away_seconds += u32::try_from(TICK_INTERVAL_SECONDS).unwrap_or(5);
        }
    } else {
        if s.pending_time_away_seconds > 0 {
            let credit = s.pending_time_away_seconds;
            let _ = store::upsert_aggregate(app, |a| {
                a.time_away_after_interruptions_seconds += credit;
            });
            apply_time_away_reward(app, credit);
            s.pending_time_away_seconds = 0;
            s.interruption_acknowledged = false;
        }
        s.idle_streak_seconds = 0;
        s.active_streak_seconds += u32::try_from(TICK_INTERVAL_SECONDS).unwrap_or(5);
    }

    let _ = store::upsert_aggregate(app, |a| {
        let delta = u32::try_from(TICK_INTERVAL_SECONDS).unwrap_or(5);
        if is_idle {
            a.idle_seconds += delta;
        } else {
            a.active_seconds += delta;
            if matches!(category, AppCategory::Social) {
                a.social_seconds += delta;
            }
        }
    });

    // Cats with earned skills slowly take care of themselves between ticks.
    if let Ok(Some(mut cat)) = store::read_cat(app) {
        if !cat.skills.is_empty() {
            cat_state::apply_autonomous_decay(
                &mut cat,
                u32::try_from(TICK_INTERVAL_SECONDS).unwrap_or(5),
            );
            let _ = store::write_cat(app, &cat);
        }
    }

    if is_idle {
        return;
    }

    // Demo mode: scheduler keeps tracking activity for aggregates and cat
    // evolution, but never auto-fires interruptions. The cat is summoned
    // exclusively via global shortcut (Cmd+Ctrl+Opt+P) so the demo timing
    // is deterministic and on-cue.
    if settings.demo_mode {
        return;
    }

    if s.active_streak_seconds < settings.grace_period_seconds {
        return;
    }

    let elapsed_since_last = s
        .last_interruption_at
        .map_or(u64::from(s.next_interruption_due_in_seconds) + 1, |t| {
            t.elapsed().as_secs()
        });

    if elapsed_since_last < u64::from(s.next_interruption_due_in_seconds) {
        return;
    }

    let aggregate_after = store::upsert_aggregate(app, |a| {
        a.interruptions += 1;
    })
    .ok();
    let task_events = store::read_task_events(app).unwrap_or_default();
    let today = chrono::Utc::now().date_naive();
    let today_completed = u32::try_from(
        task_events
            .iter()
            .filter(|e| e.completed && e.created_at.date_naive() == today)
            .count(),
    )
    .unwrap_or(u32::MAX);
    let today_dismissed = u32::try_from(
        task_events
            .iter()
            .filter(|e| {
                (e.dismissed || e.marked_inaccessible) && e.created_at.date_naive() == today
            })
            .count(),
    )
    .unwrap_or(u32::MAX);

    let payload = InterruptionPayload {
        source: InterruptionSource::Scheduler,
        active_app: foreground.clone(),
        active_app_category: category,
        time_of_day_label: current_time_of_day(),
        active_streak_seconds: s.active_streak_seconds,
        today_active_seconds: aggregate_after.as_ref().map_or(0, |a| a.active_seconds),
        today_social_seconds: aggregate_after.as_ref().map_or(0, |a| a.social_seconds),
        today_interruptions: aggregate_after.as_ref().map_or(0, |a| a.interruptions),
        today_completed,
        today_dismissed,
    };
    log::info!(
        "[activity] firing interruption: app={foreground:?} category={category:?} streak={}s",
        s.active_streak_seconds
    );
    if let Err(error) = app.emit(INTERRUPTION_REQUESTED_EVENT, payload.clone()) {
        log::warn!("[activity] failed to emit interruption event: {error}");
        return;
    }
    s.last_interruption_at = Some(Instant::now());
    s.next_interruption_due_in_seconds = random_window_seconds(
        settings.interruption_window_min_seconds,
        settings.interruption_window_max_seconds,
        matches!(category, AppCategory::Social),
    );
    s.interruption_acknowledged = true;
}

#[tauri::command]
pub fn request_interruption<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let foreground = foreground::current_foreground_app();
    let settings = store::read_settings(&app).map_err(|e| e.to_string())?;
    let category = foreground.as_ref().map_or(AppCategory::Other, |fg| {
        classifier::classify(fg, &settings.social_apps_extra)
    });
    let aggregate_after = store::upsert_aggregate(&app, |a| {
        a.interruptions += 1;
    })
    .ok();
    let task_events = store::read_task_events(&app).unwrap_or_default();
    let today = chrono::Utc::now().date_naive();
    let today_completed = u32::try_from(
        task_events
            .iter()
            .filter(|e| e.completed && e.created_at.date_naive() == today)
            .count(),
    )
    .unwrap_or(u32::MAX);
    let today_dismissed = u32::try_from(
        task_events
            .iter()
            .filter(|e| {
                (e.dismissed || e.marked_inaccessible) && e.created_at.date_naive() == today
            })
            .count(),
    )
    .unwrap_or(u32::MAX);

    let payload = InterruptionPayload {
        source: InterruptionSource::DemoTrigger,
        active_app: foreground,
        active_app_category: category,
        time_of_day_label: current_time_of_day(),
        // Manual triggers don't carry a meaningful active streak — the user
        // chose to summon the cat regardless of how long they'd been at it.
        active_streak_seconds: 0,
        today_active_seconds: aggregate_after.as_ref().map_or(0, |a| a.active_seconds),
        today_social_seconds: aggregate_after.as_ref().map_or(0, |a| a.social_seconds),
        today_interruptions: aggregate_after.as_ref().map_or(0, |a| a.interruptions),
        today_completed,
        today_dismissed,
    };
    app.emit(INTERRUPTION_REQUESTED_EVENT, payload)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn current_foreground<R: Runtime>(_app: AppHandle<R>) -> Option<ForegroundApp> {
    foreground::current_foreground_app()
}

/// Whether the app has been granted macOS Accessibility access. Without
/// this, window titles and browser URLs won't be readable; the bundle-id +
/// app-name signals still work.
#[cfg(target_os = "macos")]
#[tauri::command]
pub fn accessibility_trusted() -> bool {
    accessibility::is_trusted()
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn accessibility_trusted() -> bool {
    true
}

const TIME_AWAY_TIERS_SECONDS: [(u32, &str); 3] = [
    (10 * 60, "spent ten minutes thriving without you"),
    (30 * 60, "found a sunbeam and napped while you were away"),
    (
        2 * 60 * 60,
        "had a small adventure and brought back a story",
    ),
];

const STORY_LINES_PER_AWAY_BLOCK: &[&str] = &[
    "stretched into a perfect loaf",
    "watched a bug for an unreasonable amount of time",
    "knocked something off a shelf with extreme intent",
    "rolled belly-up for thirty unbroken seconds",
    "found the warmest square inch in the room",
];

fn apply_time_away_reward<R: Runtime>(app: &AppHandle<R>, credit_seconds: u32) {
    let Ok(Some(mut cat)) = store::read_cat(app) else {
        return;
    };
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    let bump = (f64::from(credit_seconds) / (60.0 * 60.0)) as f32;
    let previous_level = cat.independence_level;
    cat.independence_level = (cat.independence_level + bump).min(1.0);
    cat.needs.attention = (cat.needs.attention - 0.05).max(0.0);
    cat.needs.boredom = (cat.needs.boredom - 0.05).max(0.0);

    if let Some(tier_text) = TIME_AWAY_TIERS_SECONDS
        .iter()
        .rev()
        .find(|(threshold, _)| credit_seconds >= *threshold)
        .map(|(_, text)| *text)
    {
        cat.story_events.push(crate::model::StoryEvent {
            id: format!("se_{}", uuid::Uuid::new_v4()),
            at: chrono::Utc::now(),
            text: format!("{} {tier_text}.", cat.name),
        });
    } else if credit_seconds >= 60 {
        let idx = (credit_seconds as usize) % STORY_LINES_PER_AWAY_BLOCK.len();
        if let Some(line) = STORY_LINES_PER_AWAY_BLOCK.get(idx) {
            cat.story_events.push(crate::model::StoryEvent {
                id: format!("se_{}", uuid::Uuid::new_v4()),
                at: chrono::Utc::now(),
                text: format!("{} {line} while you were gone.", cat.name),
            });
        }
    }

    if cat.story_events.len() > 50 {
        let excess = cat.story_events.len() - 50;
        cat.story_events.drain(0..excess);
    }

    if (previous_level * 4.0).floor() < (cat.independence_level * 4.0).floor() {
        log::info!(
            "[activity] cat independence tier crossed: {previous_level:.2} -> {:.2}",
            cat.independence_level
        );
    }

    if let Err(error) = store::write_cat(app, &cat) {
        log::warn!("[activity] failed to persist cat after time-away credit: {error}");
    }
}
