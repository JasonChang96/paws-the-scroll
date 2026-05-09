// Some types are scaffolded here for milestones M2+ (OpenAI, overlay state,
// cat needs) and intentionally unused at M0. Allow dead code at module scope
// rather than dotting the file with attribute spam.
#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatType {
    OrangeFat,
    #[default]
    Void,
    ScrunglyStreet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatTone {
    Gentle,
    Sassy,
    Dramatic,
    Chaotic,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StuckPattern {
    Doomscroll,
    Paralysis,
    Isolation,
    Avoidance,
    Overwhelm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mobility {
    Low,
    #[default]
    Light,
    Moderate,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Environment {
    Desk,
    Bedroom,
    Office,
    Public,
    Shared,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::enum_variant_names)]
pub enum TaskBoundary {
    NoFood,
    NoLoudMovement,
    NoLeavingRoom,
    NoOutside,
    NoSocialEmbarrassment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatNeed {
    Hungry,
    Bored,
    Lonely,
    DirtyLitter,
    Play,
    Attention,
    Dramatic,
    CursedFind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskCategory {
    Movement,
    Hydration,
    Environment,
    Food,
    Stretching,
    Grounding,
    TaskInit,
}

/// Bucketed independence level. 4 tiers map cleanly to the visual evolution
/// stops in the portrait prompt; using an enum here lets every downstream
/// `match` be exhaustive instead of trusting a `u8` to stay in [0, 3].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndependenceTier {
    #[default]
    Tier0,
    Tier1,
    Tier2,
    Tier3,
}

impl IndependenceTier {
    /// The single source of truth for level→tier bucketing. Anyone needing a
    /// tier (cache key, prompt slot, UI badge) goes through this.
    pub fn from_level(level: f32) -> Self {
        if level >= 0.75 {
            IndependenceTier::Tier3
        } else if level >= 0.5 {
            IndependenceTier::Tier2
        } else if level >= 0.25 {
            IndependenceTier::Tier1
        } else {
            IndependenceTier::Tier0
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatMood {
    #[default]
    Content,
    Smug,
    Sulky,
    Excited,
    Dramatic,
    Sleepy,
    Affectionate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub goals: Vec<String>,
    pub stuck_patterns: Vec<StuckPattern>,
    pub preferred_tone: CatTone,
    pub mobility_constraints: Mobility,
    pub environment_constraints: Vec<Environment>,
    pub task_boundaries: Vec<TaskBoundary>,
    pub interruption_intensity: u8,
    pub ai_enabled: bool,
    /// Free-form per-step notes the user wrote alongside the chip/card
    /// picks. Each defaults to empty string when missing on disk so older
    /// profiles still deserialize cleanly. The AI task-prompt builder
    /// folds these into the user's context so the cat picks tasks
    /// shaped by what the user said in their own words.
    #[serde(default)]
    pub goals_notes: String,
    #[serde(default)]
    pub stuck_patterns_notes: String,
    #[serde(default)]
    pub tone_notes: String,
    #[serde(default)]
    pub mobility_notes: String,
    #[serde(default)]
    pub environment_notes: String,
    #[serde(default)]
    pub task_boundaries_notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CatNeeds {
    pub hunger: f32,
    pub boredom: f32,
    pub loneliness: f32,
    pub dirty_litter: f32,
    pub play_drive: f32,
    pub attention: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub acquired_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryEvent {
    pub id: String,
    pub at: DateTime<Utc>,
    pub text: String,
}

/// Skills the cat can earn from streak milestones. Each variant maps 1:1 to
/// a passive-decay rule in `cat_state::apply_autonomous_decay` and a visual
/// cue in `cat_state::skill_visual_hints`. Adding a variant is a compile
/// error at every consumer — exactly what we want.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillId {
    OccasionalSelfFeeding,
    IndependentPlay,
    SelfGrooming,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_field_names)]
pub struct Cat {
    pub id: String,
    #[serde(rename = "type")]
    pub cat_type: CatType,
    pub name: String,
    pub visible_traits: Vec<String>,
    pub hidden_traits: Vec<String>,
    pub needs: CatNeeds,
    pub mood: CatMood,
    pub independence_level: f32,
    pub skills: Vec<SkillId>,
    pub items: Vec<CatItem>,
    pub story_events: Vec<StoryEvent>,
    pub portrait_path: Option<String>,
    /// True when `portrait_path` still points at one of the embedded base
    /// PNGs (mango/pluto/bean) — those ship with transparent backgrounds
    /// already, so the frontend can skip the bg-removal pass entirely.
    /// Flips to false after any gpt-image-2 regeneration.
    #[serde(default = "portrait_is_base_default")]
    pub portrait_is_base: bool,
}

fn portrait_is_base_default() -> bool {
    // Default true so old persisted cats (pre this field) are treated as
    // base — worst case we skip a strip on a previously-regen'd portrait,
    // user sees the opaque bg until the next regen overwrites it.
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActivityAggregate {
    pub date: String,
    pub active_seconds: u32,
    pub idle_seconds: u32,
    pub social_seconds: u32,
    pub focus_seconds: u32,
    pub interruptions: u32,
    pub tasks_completed: u32,
    pub rerolls: u32,
    pub dismissals: u32,
    pub time_away_after_interruptions_seconds: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskSource {
    Ai,
    Fallback,
    DemoTrigger,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEvent {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub source: TaskSource,
    pub category: TaskCategory,
    pub difficulty: u8,
    pub app_category: Option<String>,
    pub reroll_index: u8,
    pub completed: bool,
    pub dismissed: bool,
    pub marked_inaccessible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub openai_api_key: Option<String>,
    pub demo_mode: bool,
    pub grace_period_seconds: u32,
    pub idle_threshold_seconds: u32,
    pub interruption_window_min_seconds: u32,
    pub interruption_window_max_seconds: u32,
    pub social_apps_extra: Vec<String>,
    pub onboarding_complete: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            openai_api_key: None,
            demo_mode: false,
            grace_period_seconds: 15 * 60,
            idle_threshold_seconds: 60,
            interruption_window_min_seconds: 3 * 60,
            interruption_window_max_seconds: 8 * 60,
            social_apps_extra: Vec::new(),
            onboarding_complete: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayMode {
    Companion,
    Interruption,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedTask {
    pub title: String,
    pub instruction: String,
    pub category: TaskCategory,
    pub difficulty: u8,
    pub estimated_seconds: u32,
    pub requires_items: bool,
    pub requires_leaving_room: bool,
    pub mobility_level: Mobility,
    pub fallback_safe: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedTaskBundle {
    pub cat_line: String,
    pub need: CatNeed,
    pub task: GeneratedTask,
    pub completion_line: String,
    pub safety_notes: Vec<String>,
}
