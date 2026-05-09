#![allow(dead_code)]
//! Pure-ish functions that translate user behavior into changes on the
//! `Cat` record. Keeping the math here means the activity scheduler,
//! interruption flow, and dashboard all reach for one source of truth
//! instead of each redoing the math inline.
//!
//! Three flavors of evolution:
//!  - **Per-task rewards**: completing a task lowers the matching need set
//!    and lifts mood; dismissing it sulks the cat.
//!  - **Streak skills**: counting distinct days with ≥1 completion in the
//!    task-event log unlocks tiered skills (day 7, 14, 30) that change how
//!    the cat behaves *and* how it looks.
//!  - **Autonomous decay**: skills the cat has earned slowly drain matching
//!    needs even when the user does nothing — the cat caring for itself.

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::Digest;

use crate::model::{
    Cat, CatMood, CatNeeds, IndependenceTier, SkillId, StoryEvent, TaskCategory, TaskEvent,
};

/// Outcome the user picked for an interruption task. Used by
/// `apply_task_outcome` to decide which mutations to apply.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskOutcome {
    Completed,
    Dismissed,
    Inaccessible,
}

/// Result of applying an outcome — the frontend uses this to decide whether
/// to kick off a fresh portrait generation and what to surface in the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeEffect {
    pub regen_portrait: bool,
    pub unlocked_skills: Vec<SkillId>,
    pub streak_days: u32,
    pub previous_portrait_signature: String,
    pub current_portrait_signature: String,
}

/// Map a task category to (`need_field`, decrement) pairs. Completing one
/// satisfies the cat in the same shape the prompt sells the task as.
fn need_decrements_for_category(category: TaskCategory) -> &'static [(NeedField, f32)] {
    match category {
        TaskCategory::Movement => &[(NeedField::Boredom, 0.30), (NeedField::PlayDrive, 0.25)],
        TaskCategory::Hydration => &[(NeedField::Attention, 0.15), (NeedField::Boredom, 0.10)],
        TaskCategory::Environment => {
            &[(NeedField::DirtyLitter, 0.30), (NeedField::Attention, 0.10)]
        }
        TaskCategory::Food => &[(NeedField::Hunger, 0.45)],
        TaskCategory::Stretching => &[(NeedField::Boredom, 0.20), (NeedField::PlayDrive, 0.20)],
        TaskCategory::Grounding => &[(NeedField::Loneliness, 0.25), (NeedField::Attention, 0.20)],
        TaskCategory::TaskInit => &[(NeedField::Attention, 0.20), (NeedField::Boredom, 0.10)],
    }
}

/// Field selector so the (field, value) pairs above stay strongly typed.
/// `apply_delta` is the only writer to `CatNeeds` outside this module.
#[derive(Debug, Clone, Copy)]
enum NeedField {
    Hunger,
    Boredom,
    Loneliness,
    DirtyLitter,
    PlayDrive,
    Attention,
}

impl NeedField {
    fn apply_delta(self, needs: &mut CatNeeds, delta: f32) {
        let slot = match self {
            NeedField::Hunger => &mut needs.hunger,
            NeedField::Boredom => &mut needs.boredom,
            NeedField::Loneliness => &mut needs.loneliness,
            NeedField::DirtyLitter => &mut needs.dirty_litter,
            NeedField::PlayDrive => &mut needs.play_drive,
            NeedField::Attention => &mut needs.attention,
        };
        *slot = (*slot + delta).clamp(0.0, 1.0);
    }
}

/// Apply a task outcome to the cat in-place. Returns metadata the caller
/// uses to drive UI feedback (regen portrait, surface unlocked skills).
pub fn apply_task_outcome(
    cat: &mut Cat,
    category: TaskCategory,
    outcome: TaskOutcome,
    history: &[TaskEvent],
) -> OutcomeEffect {
    let previous_signature = portrait_signature(cat);

    match outcome {
        TaskOutcome::Completed => {
            for (field, drop) in need_decrements_for_category(category) {
                field.apply_delta(&mut cat.needs, -drop);
            }
            push_story(
                cat,
                &format!(
                    "{} watched you finish a {} task and looks pleased.",
                    cat.name,
                    category_label(category)
                ),
            );
        }
        TaskOutcome::Dismissed => {
            // Dismissal isn't free — a couple of needs creep up. Kept small
            // so the cat never spirals into a state the user can't recover
            // from with one good task later.
            cat.needs.attention = (cat.needs.attention + 0.05).clamp(0.0, 1.0);
            cat.needs.boredom = (cat.needs.boredom + 0.05).clamp(0.0, 1.0);
        }
        TaskOutcome::Inaccessible => {
            // No need penalty — user told us this category doesn't fit.
            // Just record it; the cat treats it as neutral.
        }
    }

    cat.mood = derive_mood(cat, outcome);

    let streak_days = distinct_completion_days(history);
    let unlocked_skills = unlock_skills_from_streak(cat, streak_days);

    let current_signature = portrait_signature(cat);
    let regen_portrait = previous_signature != current_signature;

    OutcomeEffect {
        regen_portrait,
        unlocked_skills,
        streak_days,
        previous_portrait_signature: previous_signature,
        current_portrait_signature: current_signature,
    }
}

fn category_label(category: TaskCategory) -> &'static str {
    match category {
        TaskCategory::Movement => "movement",
        TaskCategory::Hydration => "hydration",
        TaskCategory::Environment => "tidying",
        TaskCategory::Food => "food",
        TaskCategory::Stretching => "stretching",
        TaskCategory::Grounding => "grounding",
        TaskCategory::TaskInit => "starting",
    }
}

/// Pick a mood from the current cat state. We prefer state-driven moods so
/// the portrait reflects what's actually true rather than a hardcoded
/// "smug on completion" rule.
fn derive_mood(cat: &Cat, outcome: TaskOutcome) -> CatMood {
    let aggregate_need = cat.needs.hunger
        + cat.needs.boredom
        + cat.needs.loneliness
        + cat.needs.dirty_litter
        + cat.needs.play_drive
        + cat.needs.attention;
    let highest_need = [
        cat.needs.hunger,
        cat.needs.boredom,
        cat.needs.loneliness,
        cat.needs.dirty_litter,
        cat.needs.play_drive,
        cat.needs.attention,
    ]
    .iter()
    .copied()
    .fold(0.0_f32, f32::max);

    match outcome {
        TaskOutcome::Dismissed => CatMood::Sulky,
        TaskOutcome::Inaccessible => CatMood::Content,
        TaskOutcome::Completed => {
            if aggregate_need < 0.6 && cat.independence_level >= 0.5 {
                CatMood::Smug
            } else if aggregate_need < 0.8 {
                CatMood::Affectionate
            } else if highest_need > 0.85 {
                CatMood::Dramatic
            } else {
                CatMood::Content
            }
        }
    }
}

fn push_story(cat: &mut Cat, text: &str) {
    cat.story_events.push(StoryEvent {
        id: format!("se_{}", uuid::Uuid::new_v4()),
        at: Utc::now(),
        text: text.to_owned(),
    });
    if cat.story_events.len() > 50 {
        let excess = cat.story_events.len() - 50;
        cat.story_events.drain(0..excess);
    }
}

/// Count distinct UTC days with at least one completed task. Not strictly
/// consecutive — life happens, missing one day shouldn't reset progress.
/// The cat is forgiving. Skill thresholds (7/14/30) read this directly.
pub fn distinct_completion_days(history: &[TaskEvent]) -> u32 {
    let days: BTreeSet<chrono::NaiveDate> = history
        .iter()
        .filter(|e| e.completed)
        .map(|e| e.created_at.date_naive())
        .collect();
    u32::try_from(days.len()).unwrap_or(u32::MAX)
}

/// If the streak crossed a tier the cat doesn't already know, append the
/// matching skill and emit a story event. Returns the newly unlocked skill
/// IDs so the UI can celebrate.
fn unlock_skills_from_streak(cat: &mut Cat, streak_days: u32) -> Vec<SkillId> {
    let mut unlocked = Vec::new();
    for (threshold, skill, story_text) in [
        (
            7,
            SkillId::OccasionalSelfFeeding,
            "learned to find a snack on their own once in a while",
        ),
        (
            14,
            SkillId::IndependentPlay,
            "figured out how to play alone without falling apart",
        ),
        (
            30,
            SkillId::SelfGrooming,
            "started keeping themselves immaculate without being asked",
        ),
    ] {
        if streak_days >= threshold && !cat.skills.contains(&skill) {
            cat.skills.push(skill);
            unlocked.push(skill);
            push_story(cat, &format!("{} {story_text}.", cat.name));
        }
    }
    unlocked
}

/// Apply autonomous need decay based on the skills the cat has earned.
/// Called from the activity scheduler on every tick. Each skill drains one
/// matching need at a slow rate — the cat caring for itself, occasionally.
pub fn apply_autonomous_decay(cat: &mut Cat, elapsed_seconds: u32) {
    if cat.skills.is_empty() {
        return;
    }
    // Scale the decay so a fully-skilled cat with all three skills doesn't
    // erase its needs in one tick. Roughly: ~0.01 per minute per skill.
    let per_second_rate = 0.01_f32 / 60.0;
    #[allow(clippy::cast_precision_loss)]
    let elapsed = elapsed_seconds as f32;

    for skill in &cat.skills {
        match skill {
            SkillId::OccasionalSelfFeeding => {
                NeedField::Hunger.apply_delta(&mut cat.needs, -per_second_rate * elapsed);
            }
            SkillId::IndependentPlay => {
                NeedField::Boredom.apply_delta(&mut cat.needs, -per_second_rate * elapsed);
                NeedField::PlayDrive.apply_delta(&mut cat.needs, -per_second_rate * elapsed);
            }
            SkillId::SelfGrooming => {
                NeedField::DirtyLitter.apply_delta(&mut cat.needs, -per_second_rate * elapsed);
            }
        }
    }
}

/// Visual hints derived from skills, fed into the portrait prompt so the
/// generated cat actually *looks* like it has earned them.
pub fn skill_visual_hints(skills: &[SkillId]) -> Vec<&'static str> {
    skills
        .iter()
        .map(|skill| match skill {
            SkillId::OccasionalSelfFeeding => {
                "with a tiny self-caught morsel nearby, looking proud of itself"
            }
            SkillId::IndependentPlay => "a small toy at its paws, mid-play",
            SkillId::SelfGrooming => "immaculately groomed, fluffy and fresh",
        })
        .collect()
}

/// Stable hash of the cat's currently-earned skill set. Feeds into the
/// portrait cache key so a cat that just learned a new skill gets a fresh
/// portrait instead of reusing the old one.
pub fn skill_set_hash(skills: &[SkillId]) -> String {
    let mut sorted = skills.to_vec();
    sorted.sort();
    let mut hasher = sha2::Sha256::new();
    for skill in &sorted {
        // Use the snake_case wire form so the hash stays stable across
        // refactors that rename the Rust variant identifiers.
        let label = serde_json::to_value(skill)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default();
        hasher.update(label.as_bytes());
        hasher.update(b"\0");
    }
    let digest = hasher.finalize();
    let hex: String = digest.iter().take(8).fold(String::new(), |mut acc, byte| {
        use std::fmt::Write;
        let _ = write!(acc, "{byte:02x}");
        acc
    });
    hex
}

/// Compact signature used to detect whether the cat's *visual* state has
/// changed (mood + tier + skills) — any flip means the portrait is stale
/// and should be regenerated.
fn portrait_signature(cat: &Cat) -> String {
    let mood = serde_json::to_value(cat.mood)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default();
    let tier = IndependenceTier::from_level(cat.independence_level);
    let tier_label = serde_json::to_value(tier)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default();
    let skills = skill_set_hash(&cat.skills);
    format!("{mood}:{tier_label}:{skills}")
}

/// Wrapper struct for the Tauri command — keeps the call site clean and
/// gives the frontend a single object to read.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomePayload {
    pub category: TaskCategory,
    pub outcome: TaskOutcome,
    pub completed_at: DateTime<Utc>,
}
