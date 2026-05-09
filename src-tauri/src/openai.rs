//! `OpenAI` client for cat dialogue, behavioral-activation tasks, completion
//! reactions, non-use rewards, and cat sprite generation.
//!
//! All traffic flows through Rust so the API key never enters the webview.
//! The frontend passes a `TaskContext` (user profile slice + cat state +
//! environment hints + reroll counter) and gets a fully-validated
//! `GeneratedTaskBundle` back, or a soft error on guardrail failure.

#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine;
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Runtime};
use tokio::sync::Mutex as AsyncMutex;

use crate::cat_bases;
use crate::cat_state;
use crate::image_cache;
use crate::model::{
    Cat, CatMood, CatNeed, CatNeeds, CatPortrait, CatTone, CatType, Environment,
    GeneratedTaskBundle, IndependenceTier, Mobility, PortraitPurpose, SkillId, StuckPattern,
    TaskBoundary, TaskCatalogueEntry, TaskCategory, TaskEvent,
};
use crate::store;

const OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
const CHAT_MODEL: &str = "gpt-5.5";
const IMAGE_MODEL: &str = "gpt-image-2";
const TASK_CATALOGUE_TARGET_SIZE: usize = 100;
const TASK_CATALOGUE_REFILL_BATCH: usize = 5;
const PORTRAIT_CATALOGUE_CAP_PER_CAT: usize = 24;
const CORE_PORTRAIT_MOODS: [CatMood; 14] = [
    CatMood::Content,
    CatMood::Peckish,
    CatMood::Hungry,
    CatMood::Lonely,
    CatMood::Restless,
    CatMood::Playful,
    CatMood::Unkempt,
    CatMood::Demanding,
    CatMood::Smug,
    CatMood::Sulky,
    CatMood::Excited,
    CatMood::Dramatic,
    CatMood::Sleepy,
    CatMood::Affectionate,
];

/// Per-path async mutex used to dedupe concurrent `generate_portrait` calls
/// targeting the same cache key. The disk-freshness check only catches
/// already-completed generations; without this, a pre-gen still in flight
/// when the user clicks "I did it" would let `regen_cat_portrait` kick off
/// a third duplicate API call. We hold this lock across the API call + write
/// so any concurrent caller for the same path waits and then sees the freshly
/// written file via the freshness window.
fn portrait_locks() -> &'static AsyncMutex<HashMap<PathBuf, Arc<AsyncMutex<()>>>> {
    static LOCKS: OnceLock<AsyncMutex<HashMap<PathBuf, Arc<AsyncMutex<()>>>>> = OnceLock::new();
    LOCKS.get_or_init(|| AsyncMutex::new(HashMap::new()))
}

async fn portrait_lock_for(path: &Path) -> Arc<AsyncMutex<()>> {
    let mut map = portrait_locks().lock().await;
    map.entry(path.to_path_buf())
        .or_insert_with(|| Arc::new(AsyncMutex::new(())))
        .clone()
}

/// Compact context the frontend passes for task generation. Field names match
/// the PRD §9 "AI Context Inputs" list.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskContext {
    pub goals: Vec<String>,
    pub stuck_patterns: Vec<StuckPattern>,
    pub mobility: Mobility,
    pub environment: Vec<Environment>,
    pub task_boundaries: Vec<TaskBoundary>,
    pub cat_type: CatType,
    pub cat_tone: CatTone,
    pub cat_mood: CatMood,
    #[serde(default)]
    pub cat_needs: CatNeeds,
    #[serde(default)]
    pub primary_cat_need: Option<CatNeed>,
    #[serde(default)]
    pub primary_cat_need_level: f32,
    pub cat_visible_traits: Vec<String>,
    pub cat_hidden_traits: Vec<String>,
    pub current_active_app: Option<String>,
    pub current_active_app_category: Option<String>,
    pub current_window_title: Option<String>,
    pub current_browser_url: Option<String>,
    pub time_of_day_label: Option<String>,
    pub reroll_index: u8,
    pub recent_completed_categories: Vec<String>,
    pub recent_dismissed_categories: Vec<String>,
    pub want_fallback: bool,
    /// Activity-tracker signals from `InterruptionPayload`. Default 0 so old
    /// callers/payloads keep deserializing.
    #[serde(default)]
    pub active_streak_seconds: u32,
    #[serde(default)]
    pub today_active_seconds: u32,
    #[serde(default)]
    pub today_social_seconds: u32,
    #[serde(default)]
    pub today_interruptions: u32,
    #[serde(default)]
    pub today_completed: u32,
    #[serde(default)]
    pub today_dismissed: u32,
    /// Free-form notes the user wrote in their own words during onboarding.
    /// Empty strings are skipped when building the prompt.
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
pub struct PortraitRequest {
    pub cat_id: String,
    pub cat_type: CatType,
    pub mood: CatMood,
    pub independence_tier: IndependenceTier,
    pub accessory_set_hash: String,
    #[serde(default)]
    pub variant_index: Option<u8>,
    /// Skill IDs the cat has earned. The prompt builder turns these into
    /// visual cues so the cat looks the part.
    #[serde(default)]
    pub skills: Vec<SkillId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortraitResponse {
    pub path: String,
    pub cached: bool,
    pub background_removed: bool,
}

const PORTRAIT_NEEDS_STRIP_EVENT: &str = "cat-portrait-needs-strip";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortraitNeedsStripPayload {
    pub raw_path: String,
    pub display_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionReaction {
    pub cat_line: String,
    pub mood_after: CatMood,
    pub suggest_step_away: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonUseRewardBundle {
    pub story_event: String,
    pub item_acquired: Option<NonUseItem>,
    pub skill_acquired: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonUseItem {
    pub name: String,
    pub description: String,
}

/// Single shared HTTP client. `reqwest::Client` keeps a connection pool
/// internally and uses `Arc` under the hood so cloning is cheap. Sharing
/// across calls means TLS handshake to api.openai.com happens once per
/// app run instead of every request — that's ~150-300ms saved on every
/// chat / image call after the first.
fn http_client() -> Result<Client> {
    use std::sync::OnceLock;
    static SHARED: OnceLock<Client> = OnceLock::new();
    if let Some(client) = SHARED.get() {
        return Ok(client.clone());
    }
    let client = Client::builder()
        .timeout(Duration::from_mins(1))
        // Generous pool — image + chat calls in flight together.
        .pool_idle_timeout(Duration::from_mins(1))
        .pool_max_idle_per_host(8)
        .build()
        .context("failed to build OpenAI HTTP client")?;
    let _ = SHARED.set(client.clone());
    Ok(client)
}

fn require_key<R: Runtime>(app: &AppHandle<R>) -> Result<String> {
    let settings = store::read_settings(app)?;
    let key = settings
        .openai_api_key
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| {
            anyhow!("OpenAI API key not configured — open Settings to add one before continuing")
        })?;
    Ok(key)
}

/// "1h 23m" / "47m" / "30s" — short, scannable strings the model can quote
/// directly without doing arithmetic itself.
fn humanize_seconds(seconds: u32) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    if hours > 0 {
        if minutes > 0 {
            format!("{hours}h {minutes}m")
        } else {
            format!("{hours}h")
        }
    } else if minutes > 0 {
        format!("{minutes}m")
    } else {
        format!("{seconds}s")
    }
}

fn cat_need_label(need: CatNeed) -> &'static str {
    match need {
        CatNeed::Hungry => "hungry",
        CatNeed::Bored => "bored",
        CatNeed::Lonely => "lonely",
        CatNeed::DirtyLitter => "dirty litter",
        CatNeed::Play => "wants play",
        CatNeed::Attention => "wants attention",
        CatNeed::Dramatic => "dramatic",
        CatNeed::CursedFind => "found something cursed",
    }
}

fn need_task_bias(need: CatNeed) -> &'static str {
    match need {
        CatNeed::Hungry => {
            "If boundaries allow food tasks, choose a tiny food/feed-the-cat-coded task: get a snack, plan food, or do one small feeding-adjacent care action."
        }
        CatNeed::Lonely => {
            "Choose a task that spends a moment with the cat: look away from the screen, give the cat attention, slow blink, breathe, or sit quietly for 30-60 seconds."
        }
        CatNeed::Bored | CatNeed::Play => {
            "Choose a task with light movement or play energy: stand, stretch, shake out hands, or do one tiny playful reset."
        }
        CatNeed::DirtyLitter => {
            "Choose a small environment reset: clear one item, tidy a surface, throw away one piece of trash, or refresh the immediate space."
        }
        CatNeed::Attention | CatNeed::Dramatic | CatNeed::CursedFind => {
            "Choose a tiny attention reset: turn from the screen, touch a real object, name what is happening, or do one grounding action."
        }
    }
}

fn task_response_schema() -> serde_json::Value {
    // Responses API: `text.format` shape — flat `type` + `name` + `schema` +
    // `strict`, no inner `json_schema` wrapper that Chat Completions used.
    serde_json::json!({
        "type": "json_schema",
        "name": "PawsTheScrollTaskBundle",
        "schema": {
            "type": "object",
            "additionalProperties": false,
            "required": ["cat_line", "need", "task", "completion_line", "safety_notes"],
            "properties": {
                "cat_line": {
                    "type": "string",
                    "description": "The cat's needy/dramatic line shown alongside the task."
                },
                "need": {
                    "type": "string",
                    "enum": ["hungry","bored","lonely","dirty_litter","play","attention","dramatic","cursed_find"]
                },
                "task": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": [
                        "title","instruction","category","difficulty","estimated_seconds",
                        "requires_items","requires_leaving_room","mobility_level","fallback_safe"
                    ],
                    "properties": {
                        "title": { "type": "string", "description": "Imperative, ≤6 words." },
                        "instruction": { "type": "string", "description": "1-2 short sentences." },
                        "category": {
                            "type": "string",
                            "enum": ["movement","hydration","environment","food","stretching","grounding","task_init"]
                        },
                        "difficulty": { "type": "integer", "minimum": 1, "maximum": 3 },
                        "estimated_seconds": { "type": "integer", "minimum": 10, "maximum": 600 },
                        "requires_items": { "type": "boolean" },
                        "requires_leaving_room": { "type": "boolean" },
                        "mobility_level": { "type": "string", "enum": ["low","light","moderate","high"] },
                        "fallback_safe": { "type": "boolean" }
                    }
                },
                "completion_line": {
                    "type": "string",
                    "description": "Cat's reaction if user marks task complete."
                },
                "safety_notes": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional safety-relevant cautions; usually empty."
                }
            }
        },
        "strict": true,
    })
}

fn task_system_prompt(ctx: &TaskContext) -> String {
    // Tone variants are now all *bratty*. The user's pick shifts which
    // flavor of bratty — gentle is "exasperated friend", chaotic is
    // "feral menace" — but every cat in this app bullies the user into
    // tiny acts of care. Real cats yell at you when they're hungry; we're
    // borrowing that energy.
    let tone = match ctx.cat_tone {
        CatTone::Gentle => "exasperated but fond, like a friend who's tired of watching you stare at your laptop",
        CatTone::Sassy => "biting, deadpan, condescending in a way that lands as funny",
        CatTone::Dramatic => "theatrically furious, fainting-couch energy, treats every minute on the screen as a personal betrayal",
        CatTone::Chaotic => "feral menace, unpredictable, gleeful about your suffering, loyal underneath",
        CatTone::Unknown => "bratty and surprising — pick a flavor of mean but loving and stick with it",
    };
    let cat_kind = match ctx.cat_type {
        CatType::OrangeFat => "an orange fat cat — food-motivated, theatrical, emotionally obvious",
        CatType::Void => "a void cat — quiet, mysterious, intense, secretly affectionate",
        CatType::ScrunglyStreet => "a scrungly street cat — chaotic, scrappy, weirdly loyal",
    };
    format!(
        "You are writing in the voice of a cat that BULLIES THE USER INTO TAKING CARE OF THEMSELVES. \
         The cat is {cat_kind}. Voice: {tone}.\n\n\
         The cat is openly mean about the *situation* — the screen, the stillness, the fact that \
         the user has become furniture. The cat is NEVER mean about the person, their feelings, \
         their worth, or their symptoms. Insult the rectangle, not the human inside it.\n\n\
         Examples of the right voice:\n\
         - \"Human. You have become furniture. Stand up for ten seconds so I know you are alive.\"\n\
         - \"Disgraceful. The screen has eaten your face. Drink water before I file a missing person report.\"\n\
         - \"Pathetic, this stillness. Touch a wall. Reclaim a single inch of dignity.\"\n\
         - \"Unbelievable. Still in that chair. I am embarrassed for both of us. Stretch. Now.\"\n\
         The energy is *bullies the user into being better*. Affectionate underneath, mean on the surface, \
         comedy as the engine.\n\n\
         RULES (never break these — they're what keep the meanness fun instead of harmful):\n\
         - Task must be doable in under {max_seconds} seconds, indoors, no special items unless the user already has them in a normal home.\n\
         - Never ask the user to leave their room unless they explicitly opted in.\n\
         - Never ask about food/eating if the user has the no_food boundary.\n\
         - Never use clinical language (depression, ADHD, anxiety, burnout, etc.). The cat is not a therapist; it's a furious gremlin.\n\
         - Never insult the user's body, intelligence, worth, mental state, or feelings. Insult the *behavior of being absorbed in the screen*, not them.\n\
         - Never shame symptoms. Doomscrolling, paralysis, zoning out — those are the *enemy*, the user is the cat's PERSON.\n\
         - Match mobility level to the user's mobility constraints. A mean cat that asks for impossible movement is not funny.\n\
         - Cat line should reference the cat's NEED (hungry/bored/etc.) as the *excuse* for bullying — \"I'm bored AND you're a corpse, do something\".\n\
         - Keep titles short (≤6 words, imperative, no hedging). Keep instructions to 1-2 sentences. The cat is impatient.\n\
         - The cat is needy and bratty, but never abandons or punishes the user. The bullying is care.\n\
         {fallback_hint}",
        max_seconds = if ctx.want_fallback { 60 } else { 240 },
        fallback_hint = if ctx.want_fallback {
            "- FALLBACK MODE: the user has rerolled a lot. Get visibly tired of them and pick the easiest possible task — but lean into it (\"FINE. Bare minimum. Tap the desk.\"). No items, no unusual movement, no embarrassment. Mark `fallback_safe: true` and `mobility_level: \"light\"`."
        } else {
            ""
        }
    )
}

// Single coherent prompt builder; splitting it into sub-functions would just
// create artificial seams without making the flow easier to read.
#[allow(clippy::too_many_lines)]
fn task_user_prompt(ctx: &TaskContext, history: &[TaskEvent]) -> String {
    let mut lines = Vec::new();
    if !ctx.goals.is_empty() {
        lines.push(format!("User goals: {}", ctx.goals.join(", ")));
    }
    if !ctx.stuck_patterns.is_empty() {
        let patterns: Vec<&str> = ctx
            .stuck_patterns
            .iter()
            .map(|p| match p {
                StuckPattern::Doomscroll => "doomscrolling",
                StuckPattern::Paralysis => "paralysis",
                StuckPattern::Isolation => "isolation",
                StuckPattern::Avoidance => "avoidance",
                StuckPattern::Overwhelm => "overwhelm",
            })
            .collect();
        lines.push(format!("Stuck patterns: {}", patterns.join(", ")));
    }
    let mobility = match ctx.mobility {
        Mobility::Low => "low — seated tasks only",
        Mobility::Light => "light — short stand-up movements OK",
        Mobility::Moderate => "moderate — short walking OK",
        Mobility::High => "high — full movement OK",
    };
    lines.push(format!("Mobility: {mobility}"));
    if !ctx.environment.is_empty() {
        let envs: Vec<&str> = ctx
            .environment
            .iter()
            .map(|e| match e {
                Environment::Desk => "desk",
                Environment::Bedroom => "bedroom",
                Environment::Office => "office (people around)",
                Environment::Public => "public space",
                Environment::Shared => "shared space",
            })
            .collect();
        lines.push(format!("Environment: {}", envs.join(", ")));
    }
    if !ctx.task_boundaries.is_empty() {
        let bounds: Vec<&str> = ctx
            .task_boundaries
            .iter()
            .map(|b| match b {
                TaskBoundary::NoFood => "no food/eating tasks",
                TaskBoundary::NoLoudMovement => "no loud movement",
                TaskBoundary::NoLeavingRoom => "no leaving the room",
                TaskBoundary::NoOutside => "no going outside",
                TaskBoundary::NoSocialEmbarrassment => "nothing socially embarrassing",
            })
            .collect();
        lines.push(format!("Boundaries: {}", bounds.join(", ")));
    }
    if let Some(app) = &ctx.current_active_app {
        lines.push(format!("Active app: {app}"));
    }
    if let Some(cat) = &ctx.current_active_app_category {
        lines.push(format!("Active app category: {cat}"));
    }
    if let Some(title) = &ctx.current_window_title {
        lines.push(format!("Focused window title: {title}"));
    }
    if let Some(url) = &ctx.current_browser_url {
        lines.push(format!("Browser URL: {url}"));
    }
    if let Some(t) = &ctx.time_of_day_label {
        lines.push(format!("Time of day: {t}"));
    }
    if let Some(need) = ctx.primary_cat_need {
        lines.push(format!(
            "Cat's current strongest need: {} at {:.0}%. {}",
            cat_need_label(need),
            ctx.primary_cat_need_level.clamp(0.0, 1.0) * 100.0,
            need_task_bias(need)
        ));
    }
    lines.push(format!(
        "Need bars: hunger {:.0}%, boredom {:.0}%, loneliness {:.0}%, litter {:.0}%, play {:.0}%, attention {:.0}%.",
        ctx.cat_needs.hunger * 100.0,
        ctx.cat_needs.boredom * 100.0,
        ctx.cat_needs.loneliness * 100.0,
        ctx.cat_needs.dirty_litter * 100.0,
        ctx.cat_needs.play_drive * 100.0,
        ctx.cat_needs.attention * 100.0,
    ));
    // Activity signals — only include when *notable*. Always-on numbers
    // turn into noise the model ignores; conditional inclusion gives the
    // cat a reason to actually quote them when they're meaningful.
    if ctx.active_streak_seconds >= 60 * 60 {
        lines.push(format!(
            "User has been continuously active for {} — long stretch.",
            humanize_seconds(ctx.active_streak_seconds)
        ));
    }
    if ctx.today_social_seconds >= 30 * 60 {
        lines.push(format!(
            "Notable: today's social-app time is {}.",
            humanize_seconds(ctx.today_social_seconds)
        ));
    }
    if ctx.today_interruptions > 1 {
        lines.push(format!(
            "This is interruption #{} today (completed {}, skipped {}).",
            ctx.today_interruptions, ctx.today_completed, ctx.today_dismissed
        ));
    }
    if !ctx.recent_completed_categories.is_empty() {
        lines.push(format!(
            "Recently completed categories (avoid repeating exactly): {}",
            ctx.recent_completed_categories.join(", ")
        ));
    }
    let recent_tasks: Vec<&TaskEvent> = history
        .iter()
        .rev()
        .filter(|event| event.task_title.is_some())
        .take(12)
        .collect();
    if !recent_tasks.is_empty() {
        lines.push(String::new());
        lines.push(
            "RECENT TASKS SHOWN (avoid repeating title, premise, or exact physical action):".into(),
        );
        for event in recent_tasks {
            let title = event.task_title.as_deref().unwrap_or("");
            let instruction = event.task_instruction.as_deref().unwrap_or("");
            let outcome = if event.completed {
                "completed"
            } else if event.dismissed {
                "dismissed"
            } else if event.marked_inaccessible {
                "inaccessible"
            } else {
                "shown"
            };
            lines.push(format!(
                "- {title}: {instruction} [{outcome}, category {:?}]",
                event.category
            ));
        }
    }
    // `recent_dismissed_categories` and `today_active_seconds` dropped:
    // dismissals already bias `want_fallback`, and total active time is
    // too vague to drive a specific task choice.
    if ctx.reroll_index > 0 {
        lines.push(format!(
            "Reroll #{n} — pick something materially different and easier than previous tries.",
            n = ctx.reroll_index
        ));
    }

    // Free-form notes, emphasized so the model leans on the user's own
    // wording rather than the categorical chips. Skip empty ones.
    let freeform = [
        ("Goals (in their own words)", &ctx.goals_notes),
        ("How they tend to get stuck", &ctx.stuck_patterns_notes),
        ("How they want the cat to talk", &ctx.tone_notes),
        ("Body / mobility specifics", &ctx.mobility_notes),
        ("Where they'll be", &ctx.environment_notes),
        ("Hard nos and limits", &ctx.task_boundaries_notes),
    ];
    let freeform_lines: Vec<String> = freeform
        .iter()
        .filter_map(|(label, text)| {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(format!("{label}: {trimmed}"))
            }
        })
        .collect();
    if !freeform_lines.is_empty() {
        lines.push(String::new());
        lines.push("USER NOTES (give these more weight than the categories above):".into());
        lines.extend(freeform_lines);
    }

    lines.push("Generate one cat line, one tiny task, and a short completion line.".to_string());
    lines.join("\n")
}

/// Responses API envelope. The `output_text` shortcut isn't always present in
/// raw API responses, so we walk `output[].content[]` for the first
/// `output_text` block.
#[derive(Deserialize)]
struct ResponsesEnvelope {
    #[serde(default)]
    output_text: Option<String>,
    #[serde(default)]
    output: Vec<ResponsesItem>,
}

#[derive(Deserialize)]
struct ResponsesItem {
    #[serde(default)]
    content: Vec<ResponsesContent>,
}

#[derive(Deserialize)]
struct ResponsesContent {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(default)]
    text: Option<String>,
}

fn extract_output_text(envelope: ResponsesEnvelope) -> Option<String> {
    if let Some(text) = envelope.output_text {
        return Some(text);
    }
    envelope
        .output
        .into_iter()
        .flat_map(|item| item.content.into_iter())
        .find(|content| content.content_type == "output_text")
        .and_then(|content| content.text)
}

/// Optional knobs that aren't part of the prompt content. `seed` and
/// `temperature` are deliberately omitted — gpt-5.5 (and reasoning models in
/// general) reject sampling-control params because internal reasoning tokens
/// have their own sampling. The Responses API returns a 400 if either is
/// included, even with `reasoning.effort: "none"`.
#[derive(Debug, Clone, Default)]
struct ChatTuning {
    /// Stable identifier for the prompt prefix bucket. `OpenAI` uses this to
    /// improve cache routing for repeated calls; cat-tone-stable rerolls
    /// hit the same bucket so we save on input-token cost.
    cache_key: Option<String>,
}

async fn call_chat_json<T: serde::de::DeserializeOwned>(
    api_key: &str,
    system_prompt: &str,
    user_prompt: &str,
    schema: serde_json::Value,
    tuning: &ChatTuning,
) -> Result<T> {
    let client = http_client()?;
    // Responses API. Differences from Chat Completions:
    //   - endpoint: /v1/responses
    //   - `messages` -> `input`
    //   - system message -> `instructions` (top-level)
    //   - `response_format` -> `text.format`
    //   - `verbosity` -> `text.verbosity`
    //   - `reasoning_effort` -> `reasoning.effort`
    //   - we don't `store` reasoning state — each interruption is fresh.
    let mut body = serde_json::json!({
        "model": CHAT_MODEL,
        "instructions": system_prompt,
        "input": user_prompt,
        "text": {
            "format": schema,
            "verbosity": "low",
        },
        "reasoning": { "effort": "none" },
        "store": false,
    });
    if let Some(key) = &tuning.cache_key {
        body["prompt_cache_key"] = serde_json::Value::String(key.clone());
    }
    log::info!(
        "[openai] POST /v1/responses model={CHAT_MODEL} cache_key={:?}",
        tuning.cache_key
    );
    let started = std::time::Instant::now();
    let resp = client
        .post(format!("{OPENAI_BASE_URL}/responses"))
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .context("OpenAI responses request failed")?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .context("failed to read OpenAI responses body")?;
    log::info!(
        "[openai] /v1/responses {status} in {}ms ({} bytes)",
        started.elapsed().as_millis(),
        text.len()
    );
    if !status.is_success() {
        bail!("OpenAI responses returned {status}: {text}");
    }
    let envelope: ResponsesEnvelope = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse Responses envelope: {text}"))?;
    let content = extract_output_text(envelope)
        .ok_or_else(|| anyhow!("OpenAI responses had no output_text content"))?;
    serde_json::from_str::<T>(&content)
        .with_context(|| format!("OpenAI returned non-conforming JSON: {content}"))
}

fn validate_task(bundle: &GeneratedTaskBundle, ctx: &TaskContext) -> Result<()> {
    let task = &bundle.task;
    if task.title.trim().is_empty() {
        bail!("generated task title is empty");
    }
    if task.instruction.trim().is_empty() {
        bail!("generated task instruction is empty");
    }
    if task.estimated_seconds == 0 || task.estimated_seconds > 600 {
        bail!("generated task duration outside 10–600s range");
    }
    if task.requires_leaving_room
        && (ctx.task_boundaries.contains(&TaskBoundary::NoLeavingRoom)
            || ctx.environment.contains(&Environment::Public))
    {
        bail!("task asks user to leave the room despite boundary/environment");
    }
    if matches!(task.category, crate::model::TaskCategory::Food)
        && ctx.task_boundaries.contains(&TaskBoundary::NoFood)
    {
        bail!("task is food-related but user opted out of food tasks");
    }
    if ctx.primary_cat_need_level >= 0.6
        && matches!(ctx.primary_cat_need, Some(CatNeed::Hungry))
        && !ctx.task_boundaries.contains(&TaskBoundary::NoFood)
        && !matches!(task.category, crate::model::TaskCategory::Food)
    {
        bail!("cat is hungry; generated task should be food-related");
    }
    if ctx.primary_cat_need_level >= 0.6
        && matches!(ctx.primary_cat_need, Some(CatNeed::Lonely))
        && !matches!(task.category, crate::model::TaskCategory::Grounding)
    {
        bail!("cat is lonely; generated task should spend attention with the cat");
    }
    if mobility_strictness(task.mobility_level) > mobility_strictness(ctx.mobility) {
        bail!(
            "task mobility {:?} exceeds user mobility {:?}",
            task.mobility_level,
            ctx.mobility
        );
    }
    if ctx.want_fallback && !task.fallback_safe {
        bail!("fallback was requested but task is not fallback_safe");
    }
    Ok(())
}

fn mobility_strictness(level: Mobility) -> u8 {
    match level {
        Mobility::Low => 0,
        Mobility::Light => 1,
        Mobility::Moderate => 2,
        Mobility::High => 3,
    }
}

fn chat_tuning_for(ctx: &TaskContext) -> ChatTuning {
    let cache_key = format!(
        "paws/cat/{cat_type:?}/{tone:?}/{fallback}",
        cat_type = ctx.cat_type,
        tone = ctx.cat_tone,
        fallback = if ctx.want_fallback {
            "fallback"
        } else {
            "normal"
        },
    );
    ChatTuning {
        cache_key: Some(cache_key),
    }
}

pub async fn generate_task_with_retry<R: Runtime>(
    app: &AppHandle<R>,
    ctx: &TaskContext,
) -> Result<GeneratedTaskBundle> {
    let key = require_key(app)?;
    let system_prompt = task_system_prompt(ctx);
    let history = store::read_task_events(app).unwrap_or_default();
    let user_prompt = task_user_prompt(ctx, &history);
    let schema = task_response_schema();
    let tuning = chat_tuning_for(ctx);

    let mut last_error: Option<anyhow::Error> = None;
    for attempt in 0..3 {
        let bundle = call_chat_json::<GeneratedTaskBundle>(
            &key,
            &system_prompt,
            &user_prompt,
            schema.clone(),
            &tuning,
        )
        .await;
        match bundle {
            Ok(bundle) => match validate_task(&bundle, ctx) {
                Ok(()) => return Ok(bundle),
                Err(e) => {
                    log::warn!("[openai] guardrail rejected task on attempt {attempt}: {e}");
                    last_error = Some(e);
                }
            },
            Err(e) => {
                log::warn!("[openai] task call failed on attempt {attempt}: {e}");
                last_error = Some(e);
            }
        }
    }
    Err(last_error.unwrap_or_else(|| anyhow!("OpenAI task generation failed without error")))
}

fn task_matches_context(bundle: &GeneratedTaskBundle, ctx: &TaskContext) -> bool {
    validate_task(bundle, ctx).is_ok()
        && (ctx.primary_cat_need_level < 0.35 || Some(bundle.need) == ctx.primary_cat_need)
        && (!ctx.want_fallback || bundle.task.fallback_safe)
}

fn catalogue_sort_key(entry: &TaskCatalogueEntry) -> (u32, u32, chrono::DateTime<Utc>) {
    (
        entry.display_count,
        entry.dismissed_count + entry.inaccessible_count.saturating_mul(2),
        entry.last_shown_at.unwrap_or(entry.created_at),
    )
}

fn context_variants_for_catalogue(ctx: &TaskContext) -> Vec<TaskContext> {
    let needs = [
        CatNeed::Hungry,
        CatNeed::Bored,
        CatNeed::Lonely,
        CatNeed::DirtyLitter,
        CatNeed::Play,
        CatNeed::Attention,
    ];
    needs
        .into_iter()
        .map(|need| {
            let mut next = ctx.clone();
            next.primary_cat_need = Some(need);
            next.primary_cat_need_level = 0.75;
            next.want_fallback = false;
            next.recent_completed_categories.clear();
            next.recent_dismissed_categories.clear();
            next
        })
        .collect()
}

fn make_task_catalogue_entry(bundle: GeneratedTaskBundle) -> TaskCatalogueEntry {
    TaskCatalogueEntry {
        id: format!("task_{}", uuid::Uuid::new_v4()),
        category: bundle.task.category,
        need: bundle.need,
        mobility_level: bundle.task.mobility_level,
        fallback_safe: bundle.task.fallback_safe,
        bundle,
        created_at: Utc::now(),
        last_shown_at: None,
        display_count: 0,
        completed_count: 0,
        dismissed_count: 0,
        inaccessible_count: 0,
    }
}

async fn refill_task_catalogue<R: Runtime>(app: &AppHandle<R>, ctx: TaskContext) {
    let existing = store::read_task_catalogue(app).unwrap_or_default();
    if existing.len() >= TASK_CATALOGUE_TARGET_SIZE {
        return;
    }
    let remaining = TASK_CATALOGUE_TARGET_SIZE - existing.len();
    let count = remaining.min(TASK_CATALOGUE_REFILL_BATCH);
    let variants = context_variants_for_catalogue(&ctx);
    for index in 0..count {
        let Some(task_context) = variants.get(index % variants.len()).cloned() else {
            return;
        };
        match generate_task_with_retry(app, &task_context).await {
            Ok(bundle) => {
                let title = bundle.task.title.clone();
                let entry = make_task_catalogue_entry(bundle);
                if let Err(error) = store::append_task_catalogue_entry(app, entry) {
                    log::warn!("[openai] failed to store task catalogue entry: {error}");
                } else {
                    log::info!("[openai] task catalogue stored: {title}");
                }
            }
            Err(error) => log::warn!("[openai] task catalogue refill failed: {error}"),
        }
    }
}

#[tauri::command]
pub fn warm_task_catalogue<R: Runtime>(
    app: AppHandle<R>,
    context: TaskContext,
) -> Result<(), String> {
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        refill_task_catalogue(&app_clone, context).await;
    });
    Ok(())
}

#[tauri::command]
pub fn select_catalogue_task<R: Runtime>(
    app: AppHandle<R>,
    context: TaskContext,
) -> Result<Option<GeneratedTaskBundle>, String> {
    let mut entries = store::read_task_catalogue(&app).map_err(|e| e.to_string())?;
    entries.retain(|entry| task_matches_context(&entry.bundle, &context));
    entries.sort_by_key(catalogue_sort_key);
    let Some(selected) = entries.into_iter().next() else {
        let app_clone = app.clone();
        tauri::async_runtime::spawn(async move {
            refill_task_catalogue(&app_clone, context).await;
        });
        return Ok(None);
    };
    let selected_id = selected.id.clone();
    let bundle = selected.bundle.clone();
    let _ = store::update_task_catalogue_entry(&app, &selected_id, |entry| {
        entry.display_count = entry.display_count.saturating_add(1);
        entry.last_shown_at = Some(Utc::now());
    })
    .map_err(|e| e.to_string())?;
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        refill_task_catalogue(&app_clone, context).await;
    });
    Ok(Some(bundle))
}

#[derive(Deserialize)]
struct ImageEditEnvelope {
    data: Vec<ImageEditEntry>,
}

#[derive(Deserialize)]
struct ImageEditEntry {
    b64_json: String,
}

/// Non-streaming image *edit* against a hand-drawn base portrait. We tried
/// `stream: true, partial_images: 3` per the docs but gpt-image-2 only
/// emits a single `image_edit.completed` event for our payloads — paying
/// SSE overhead for nothing. Standard JSON response is faster and simpler.
async fn call_image_edit(
    cat_id: &str,
    api_key: &str,
    prompt: &str,
    base_image: &'static [u8],
) -> Result<Vec<u8>> {
    let client = http_client()?;
    let form = reqwest::multipart::Form::new()
        .text("model", IMAGE_MODEL.to_string())
        .text("prompt", prompt.to_string())
        .text("size", "1024x1024")
        .text("quality", "low")
        .text("background", "opaque")
        .text("output_format", "jpeg")
        .text("n", "1")
        .part(
            "image",
            reqwest::multipart::Part::bytes(base_image.to_vec())
                .file_name("base.png")
                .mime_str("image/png")
                .context("failed to set image part mime")?,
        );
    log::info!(
        "[openai] POST /v1/images/edits model={IMAGE_MODEL} cat_id={cat_id} (multipart, quality=low, jpeg)"
    );
    let started = std::time::Instant::now();
    let resp = client
        .post(format!("{OPENAI_BASE_URL}/images/edits"))
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await
        .context("OpenAI image edit request failed")?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .context("failed to read OpenAI image edit body")?;
    log::info!(
        "[openai] /v1/images/edits {status} in {}ms ({} bytes)",
        started.elapsed().as_millis(),
        text.len()
    );
    if !status.is_success() {
        bail!("OpenAI images returned {status}: {text}");
    }

    let envelope: ImageEditEnvelope = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse images/edits envelope: {text}"))?;
    let entry = envelope
        .data
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("OpenAI returned no image data"))?;
    let raw_bytes = base64::engine::general_purpose::STANDARD
        .decode(entry.b64_json.as_bytes())
        .context("failed to base64-decode generated image")?;

    // Background removal moved to the frontend (`@imgly/background-removal`)
    // which runs the segmentation model directly in the webview — no
    // Python venv, no PATH detection, no shell-out. Backend just returns
    // the raw gpt-image-2 bytes.
    Ok(raw_bytes)
}

/// Build an *edit* prompt — short, because the base image already carries
/// the breed and the visual style. We describe what should *change* from
/// the base: mood, independence, earned-skill details. The cat *character*
/// stays constant; pose and composition are left open for the model to
/// improvise.
fn portrait_prompt(req: &PortraitRequest) -> String {
    // Mood phrases tuned for gpt-image-2 to render visibly distinct frames
    // at the small companion size — bold pose + facial language, not subtle
    // microexpressions the model would average out.
    let mood = match req.mood {
        CatMood::Content => "relaxed, neutral expression, eyes forward, calm posture",
        CatMood::Peckish => "mildly hungry, expectant eyes, tiny impatient paw lift, hopeful but not frantic",
        CatMood::Hungry => "clearly hungry, pleading round eyes, mouth slightly open as if yelling for food, alert forward posture",
        CatMood::Lonely => "lonely and soft, leaning toward the viewer, ears slightly drooped, wanting company",
        CatMood::Restless => "restless and under-stimulated, tail twitching, paws ready to move, impatient energy",
        CatMood::Playful => "playful, bright eyes, pouncing crouch, tail up, mischievous and ready for a game",
        CatMood::Unkempt => "unkempt and bothered, slightly ruffled fur, offended expression, wants the space cleaned",
        CatMood::Demanding => "demanding attention, intense eye contact, one paw raised, bossy but affectionate",
        CatMood::Smug => "clearly proud and pleased, lifted chin, slow-blink half-closed eyes, faint smile, chest puffed out",
        CatMood::Sulky => "visibly disappointed, head turned slightly away, ears flattened, no eye contact, downcast eyes, hunched posture",
        CatMood::Excited => "wide bright eyes, ears perked sharply forward, mid-celebratory wiggle, mouth slightly open, alert and energized",
        CatMood::Dramatic => "exaggerated theatrical despair, paw raised toward forehead, huge round sad eyes, fainting-couch energy",
        CatMood::Sleepy => "eyes mostly closed, loafing pose, soft sleepy blink, content and dozing",
        CatMood::Affectionate => "warm cheek-rub posture, eyes squinted shut in trust, soft happy expression, clearly pleased with you",
    };
    let independence = match req.independence_tier {
        IndependenceTier::Tier0 => "needy and clingy posture",
        IndependenceTier::Tier1 => "balanced posture, looks comfortable",
        IndependenceTier::Tier2 => "confident posture, slightly independent",
        IndependenceTier::Tier3 => "fully independent, capable, slightly worldly",
    };
    let skill_hints = cat_state::skill_visual_hints(&req.skills);
    let skill_clause = if skill_hints.is_empty() {
        String::new()
    } else {
        format!(" Add: {}.", skill_hints.join("; "))
    };
    format!(
        "Same cat — same breed, fur pattern, color palette, and overall \
         identity as the source image. Pose, body language, and any small \
         accessories or props are entirely up to you — pick something fresh \
         and characterful, not a copy of the source pose. \
         {PORTRAIT_STYLE_ANCHOR} \
         Mood: {mood}; {independence}.{skill_clause} \
         Centered, full body visible, no text. \
         The cat is fully isolated on a clean transparent background."
    )
}

/// Compact style anchor included in every edit prompt. The base image carries
/// the full visual language; this short reminder just keeps edits from
/// drifting toward vector-clean or corporate-mascot territory across many
/// regenerations.
const PORTRAIT_STYLE_ANCHOR: &str = "Keep the cozy hand-painted illustration style: \
soft sketchy outlines, plush readable shapes, warm painterly shading, \
hand-crafted desktop-pet feel — never vector-clean.";

fn label_for<T: Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

fn request_skills_hash(req: &PortraitRequest) -> String {
    cat_state::skill_set_hash(&req.skills)
}

fn matching_catalogue_portrait<R: Runtime>(
    app: &AppHandle<R>,
    req: &PortraitRequest,
) -> Result<Option<CatPortrait>> {
    let skills_hash = request_skills_hash(req);
    let mut matches: Vec<CatPortrait> = store::read_cat_portraits(app)?
        .into_iter()
        .filter(|portrait| {
            portrait.cat_id == req.cat_id
                && portrait.mood == req.mood
                && portrait.independence_tier == req.independence_tier
                && portrait.skills_hash == skills_hash
                && portrait.background_removed
                && image_cache::is_png(std::path::Path::new(&portrait.path))
        })
        .collect();
    matches.sort_by_key(|portrait| (portrait.display_count, portrait.last_shown_at));
    Ok(matches.into_iter().next())
}

fn emit_portrait_needs_strip<R: Runtime>(
    app: &AppHandle<R>,
    raw_path: String,
    display_path: String,
) {
    if let Err(error) = app.emit(
        PORTRAIT_NEEDS_STRIP_EVENT,
        PortraitNeedsStripPayload {
            raw_path,
            display_path,
        },
    ) {
        log::warn!("[openai] failed to emit portrait strip request: {error}");
    }
}

fn mark_portrait_shown<R: Runtime>(app: &AppHandle<R>, path: &str) -> Result<Option<CatPortrait>> {
    store::update_cat_portrait(app, path, |portrait| {
        portrait.display_count = portrait.display_count.saturating_add(1);
        portrait.last_shown_at = Some(Utc::now());
    })
}

fn next_variant_index<R: Runtime>(app: &AppHandle<R>, req: &PortraitRequest) -> Result<u8> {
    if let Some(index) = req.variant_index {
        return Ok(index);
    }
    let skills_hash = request_skills_hash(req);
    let next = store::read_cat_portraits(app)?
        .into_iter()
        .filter(|portrait| {
            portrait.cat_id == req.cat_id
                && portrait.mood == req.mood
                && portrait.independence_tier == req.independence_tier
                && portrait.skills_hash == skills_hash
        })
        .map(|portrait| portrait.variant_index)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    Ok(next)
}

fn catalogue_key_parts(
    req: &PortraitRequest,
    cat_type_label: &str,
    mood_label: &str,
    tier_label: &str,
    skills_hash: &str,
    variant_index: u8,
) -> Vec<String> {
    vec![
        req.cat_id.clone(),
        cat_type_label.to_owned(),
        mood_label.to_owned(),
        tier_label.to_owned(),
        req.accessory_set_hash.clone(),
        skills_hash.to_owned(),
        format!("variant{variant_index}"),
    ]
}

#[allow(clippy::too_many_arguments)]
fn new_catalogue_portrait(
    req: &PortraitRequest,
    skills_hash: String,
    variant_index: u8,
    path: String,
    raw_path: Option<String>,
    purpose: PortraitPurpose,
    is_core: bool,
    background_removed: bool,
) -> CatPortrait {
    CatPortrait {
        id: format!("portrait_{}", uuid::Uuid::new_v4()),
        cat_id: req.cat_id.clone(),
        mood: req.mood,
        independence_tier: req.independence_tier,
        skills_hash,
        variant_index,
        purpose,
        path,
        raw_path,
        is_core,
        background_removed,
        generated_at: Utc::now(),
        last_shown_at: None,
        display_count: 0,
    }
}

fn enforce_portrait_catalogue_cap<R: Runtime>(app: &AppHandle<R>, cat_id: &str) -> Result<()> {
    let mut portraits = store::read_cat_portraits(app)?;
    let count_for_cat = portraits
        .iter()
        .filter(|portrait| portrait.cat_id == cat_id)
        .count();
    if count_for_cat <= PORTRAIT_CATALOGUE_CAP_PER_CAT {
        return Ok(());
    }

    let current_path = store::read_cat(app)?
        .and_then(|cat| {
            if cat.id == cat_id {
                cat.portrait_path
            } else {
                None
            }
        })
        .unwrap_or_default();
    let mut removable: Vec<CatPortrait> = portraits
        .iter()
        .filter(|portrait| portrait.cat_id == cat_id && !portrait.is_core)
        .filter(|portrait| portrait.path != current_path)
        .cloned()
        .collect();
    removable.sort_by_key(|portrait| (portrait.last_shown_at, portrait.generated_at));

    let excess = count_for_cat - PORTRAIT_CATALOGUE_CAP_PER_CAT;
    let remove_paths: Vec<String> = removable
        .into_iter()
        .take(excess)
        .map(|portrait| portrait.path)
        .collect();
    if remove_paths.is_empty() {
        return Ok(());
    }
    portraits.retain(|portrait| !remove_paths.iter().any(|path| path == &portrait.path));
    store::write_cat_portraits(app, &portraits)?;
    for path in remove_paths {
        if let Err(error) = std::fs::remove_file(&path) {
            log::warn!("[openai] failed to evict old portrait {path}: {error}");
        }
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
pub async fn generate_portrait<R: Runtime>(
    app: &AppHandle<R>,
    req: &PortraitRequest,
) -> Result<PortraitResponse> {
    if req.variant_index.is_none() {
        if let Some(portrait) = matching_catalogue_portrait(app, req)? {
            let portrait = mark_portrait_shown(app, &portrait.path)?.unwrap_or(portrait);
            log::info!(
                "[openai] portrait catalogue hit cat_id={} mood={} variant={}",
                req.cat_id,
                label_for(req.mood),
                portrait.variant_index
            );
            return Ok(PortraitResponse {
                path: portrait.path,
                cached: true,
                background_removed: portrait.background_removed,
            });
        }
    }

    let cat_type_label = label_for(req.cat_type);
    let mood_label = label_for(req.mood);
    let tier_label = label_for(req.independence_tier);
    let skills_hash = request_skills_hash(req);
    let variant_index = next_variant_index(app, req)?;
    let key_parts = catalogue_key_parts(
        req,
        &cat_type_label,
        &mood_label,
        &tier_label,
        &skills_hash,
        variant_index,
    );
    let key_refs: Vec<&str> = key_parts.iter().map(String::as_str).collect();
    let key = image_cache::make_key(&key_refs);
    let path = image_cache::display_path_for_key(app, &key)?;
    let raw_path = image_cache::raw_path_for_key(app, &key)?;

    // Serialize concurrent calls targeting the same cache key. Without this,
    // a pre-gen still in flight when the user clicks would let
    // `regen_cat_portrait` start a third duplicate API call (the disk
    // freshness check can't see in-flight work). Holding this lock across
    // the API call + write means the second caller waits, then either picks
    // up the freshly-written file via the freshness check below or — if
    // the producer failed — proceeds to generate itself.
    let lock = portrait_lock_for(&path).await;
    let _guard = lock.lock().await;

    if std::fs::metadata(&path).is_ok() {
        let path_str = path.to_string_lossy().into_owned();
        let raw_path_str = raw_path.to_string_lossy().into_owned();
        let existing = store::read_cat_portraits(app)?
            .into_iter()
            .find(|portrait| portrait.path == path_str);
        let background_removed = existing
            .as_ref()
            .is_some_and(|portrait| portrait.background_removed)
            && image_cache::is_png(&path);
        let background_removed =
            existing.is_none() && image_cache::is_png(&path) || background_removed;
        if existing.is_some() && !background_removed {
            let _ = store::update_cat_portrait(app, &path_str, |portrait| {
                portrait.background_removed = false;
            });
        }
        if existing.is_none() {
            let portrait = new_catalogue_portrait(
                req,
                skills_hash,
                variant_index,
                path_str.clone(),
                Some(raw_path_str),
                PortraitPurpose::Outcome,
                false,
                true,
            );
            let _ = store::upsert_cat_portrait(app, portrait)?;
        }
        if !background_removed {
            emit_portrait_needs_strip(app, path_str.clone(), path_str.clone());
            return Ok(PortraitResponse {
                path: path_str,
                cached: true,
                background_removed: false,
            });
        }
        return Ok(PortraitResponse {
            path: path_str,
            cached: true,
            background_removed,
        });
    }

    if std::fs::metadata(&raw_path).is_ok() {
        let raw_path_str = raw_path.to_string_lossy().into_owned();
        let path_str = path.to_string_lossy().into_owned();
        let existing = store::read_cat_portraits(app)?
            .into_iter()
            .find(|portrait| {
                portrait.path == path_str
                    || portrait.raw_path.as_deref() == Some(raw_path_str.as_str())
            });
        if existing.is_none() {
            let portrait = new_catalogue_portrait(
                req,
                skills_hash,
                variant_index,
                path_str.clone(),
                Some(raw_path_str.clone()),
                PortraitPurpose::Outcome,
                false,
                false,
            );
            let _ = store::upsert_cat_portrait(app, portrait)?;
        }
        emit_portrait_needs_strip(app, raw_path_str.clone(), path_str);
        return Ok(PortraitResponse {
            path: raw_path_str,
            cached: true,
            background_removed: false,
        });
    }

    log::info!(
        "[openai] generating portrait cat_id={} mood={mood_label} tier={tier_label} skills={skills_hash} variant={variant_index}",
        req.cat_id
    );
    let api_key = require_key(app)?;
    let prompt = portrait_prompt(req);
    let base_image = cat_bases::bytes_for(req.cat_type);
    let bytes = call_image_edit(&req.cat_id, &api_key, &prompt, base_image).await?;
    image_cache::write_cached(&raw_path, &bytes)?;
    // The bytes we just wrote include the gpt-image-2 background. If the cat
    // is currently displaying this exact path with `portrait_is_base = true`
    // (a previous run had stripped + persisted at this path), the frontend
    // would short-circuit on its next read and flash the un-stripped image
    // before regen_cat_portrait flips the flag. Invalidate proactively so
    // the strip path always runs for these fresh API bytes.
    let path_str = path.to_string_lossy().into_owned();
    let raw_path_str = raw_path.to_string_lossy().into_owned();
    if let Ok(Some(mut cat)) = store::read_cat(app) {
        if cat.portrait_is_base
            && (cat.portrait_path.as_deref() == Some(path_str.as_str())
                || cat.portrait_path.as_deref() == Some(raw_path_str.as_str()))
        {
            cat.portrait_is_base = false;
            let _ = store::write_cat(app, &cat);
        }
    }
    let portrait = new_catalogue_portrait(
        req,
        skills_hash,
        variant_index,
        path_str.clone(),
        Some(raw_path_str.clone()),
        PortraitPurpose::Outcome,
        false,
        false,
    );
    let _ = store::upsert_cat_portrait(app, portrait)?;
    emit_portrait_needs_strip(app, raw_path_str.clone(), path_str.clone());
    enforce_portrait_catalogue_cap(app, &req.cat_id)?;
    Ok(PortraitResponse {
        path: raw_path_str,
        cached: false,
        background_removed: false,
    })
}

#[tauri::command]
pub async fn generate_interruption_task(
    app: AppHandle,
    context: TaskContext,
) -> Result<GeneratedTaskBundle, String> {
    generate_task_with_retry(&app, &context)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn generate_cat_portrait(
    app: AppHandle,
    request: PortraitRequest,
) -> Result<PortraitResponse, String> {
    generate_portrait(&app, &request)
        .await
        .map_err(|e| e.to_string())
}

/// Overwrite the on-disk portrait file with bytes the frontend already
/// background-stripped (PNG with alpha). Flips `cat.portrait_is_base` so
/// future reads — including from other webviews like the dashboard —
/// short-circuit the bg-removal pass and load the already-transparent
/// file directly.
#[tauri::command]
pub async fn persist_stripped_portrait<R: Runtime>(
    app: AppHandle<R>,
    path: String,
    data_url: String,
) -> Result<(), String> {
    let mut portraits = store::read_cat_portraits(&app).map_err(|e| e.to_string())?;
    let portrait_index = portraits.iter().position(|portrait| {
        portrait.path == path || portrait.raw_path.as_deref() == Some(path.as_str())
    });
    let target_path = portrait_index
        .and_then(|index| portraits.get(index).map(|portrait| portrait.path.clone()))
        .unwrap_or_else(|| path.clone());

    let comma = data_url
        .find(',')
        .ok_or_else(|| "input is not a data URL".to_string())?;
    let b64 = &data_url[comma + 1..];
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.as_bytes())
        .map_err(|e| e.to_string())?;
    if !bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        let _ = store::update_cat_portrait(&app, &target_path, |portrait| {
            portrait.background_removed = false;
        });
        return Err(
            "background removal did not produce a PNG; leaving portrait marked unstripped".into(),
        );
    }
    std::fs::write(&target_path, &bytes).map_err(|e| e.to_string())?;
    if let Ok(Some(mut cat)) = store::read_cat(&app) {
        if cat.portrait_path.as_deref() == Some(path.as_str())
            || cat.portrait_path.as_deref() == Some(target_path.as_str())
        {
            cat.portrait_path = Some(target_path.clone());
            cat.portrait_is_base = true;
            store::write_cat(&app, &cat).map_err(|e| e.to_string())?;
        }
    }
    if let Some(index) = portrait_index {
        if let Some(portrait) = portraits.get_mut(index) {
            portrait.path.clone_from(&target_path);
            portrait.background_removed = true;
        }
        store::write_cat_portraits(&app, &portraits).map_err(|e| e.to_string())?;
    }
    let _ = store::update_cat_portrait(&app, &target_path, |portrait| {
        portrait.background_removed = true;
    })
    .map_err(|e| e.to_string())?;
    log::info!(
        "[openai] persisted stripped portrait: {} bytes at {target_path}",
        bytes.len()
    );
    Ok(())
}

#[tauri::command]
pub async fn read_portrait_bytes(path: String) -> Result<String, String> {
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    // Sniff the actual format from magic bytes — the cache might hold PNG
    // (rembg-processed) or JPEG (rembg unavailable) and the data URL needs
    // the correct mime so Tauri's WebKit decodes it.
    let mime = if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        "image/png"
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "image/jpeg"
    } else {
        "image/png"
    };
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{mime};base64,{b64}"))
}

/// Seed the initial portrait at adoption time using the embedded base image.
/// No `OpenAI` call — the hand-drawn base PNG already *is* the cat in its
/// starting state (mood=content, tier0, no skills). State-change regenerations
/// later go through `regen_cat_portrait` and the edit endpoint as usual.
#[tauri::command]
pub fn seed_initial_portrait(
    app: AppHandle,
    cat_id: String,
    cat_type: CatType,
) -> Result<PortraitResponse, String> {
    let request = PortraitRequest {
        cat_id: cat_id.clone(),
        cat_type,
        mood: CatMood::Content,
        independence_tier: IndependenceTier::Tier0,
        accessory_set_hash: "v1".into(),
        variant_index: Some(0),
        skills: Vec::new(),
    };
    let cat_type_label = label_for(request.cat_type);
    let mood_label = label_for(request.mood);
    let tier_label = label_for(request.independence_tier);
    let skills_hash = request_skills_hash(&request);
    let key_parts = catalogue_key_parts(
        &request,
        &cat_type_label,
        &mood_label,
        &tier_label,
        &skills_hash,
        0,
    );
    let key_refs: Vec<&str> = key_parts.iter().map(String::as_str).collect();
    let key = image_cache::make_key(&key_refs);
    let path = image_cache::path_for_key(&app, &key).map_err(|e| e.to_string())?;
    if image_cache::read_cached(&path).is_none() {
        image_cache::write_cached(&path, cat_bases::bytes_for(cat_type))
            .map_err(|e| e.to_string())?;
    }
    let portrait = new_catalogue_portrait(
        &request,
        skills_hash,
        0,
        path.to_string_lossy().into_owned(),
        None,
        PortraitPurpose::Core,
        true,
        true,
    );
    let _ = store::upsert_cat_portrait(&app, portrait).map_err(|e| e.to_string())?;
    Ok(PortraitResponse {
        path: path.to_string_lossy().into_owned(),
        cached: false,
        background_removed: true,
    })
}

#[tauri::command]
pub fn warm_cat_portrait_catalogue<R: Runtime>(
    app: AppHandle<R>,
    cat_id: String,
) -> Result<(), String> {
    let Some(cat) = store::read_cat(&app).map_err(|e| e.to_string())? else {
        return Ok(());
    };
    if cat.id != cat_id {
        return Ok(());
    }
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        warm_core_portraits(&app_clone, cat).await;
    });
    Ok(())
}

async fn warm_core_portraits<R: Runtime>(app: &AppHandle<R>, cat: Cat) {
    log::info!(
        "[openai] warming core portrait catalogue for cat_id={}",
        cat.id
    );
    for mood in CORE_PORTRAIT_MOODS {
        let request = PortraitRequest {
            cat_id: cat.id.clone(),
            cat_type: cat.cat_type,
            mood,
            independence_tier: IndependenceTier::from_level(cat.independence_level),
            accessory_set_hash: "v1".into(),
            variant_index: Some(0),
            skills: cat.skills.clone(),
        };
        match generate_portrait(app, &request).await {
            Ok(response) => {
                let skills_hash = request_skills_hash(&request);
                let _ = store::update_cat_portrait(app, &response.path, |portrait| {
                    portrait.purpose = PortraitPurpose::Core;
                    portrait.is_core = true;
                    portrait.skills_hash = skills_hash;
                    portrait.background_removed = response.background_removed;
                });
            }
            Err(error) => {
                log::warn!(
                    "[openai] core portrait warm failed cat_id={} mood={}: {error}",
                    cat.id,
                    label_for(mood)
                );
            }
        }
    }
}

fn queue_catalogue_enrichment<R: Runtime>(app: &AppHandle<R>, request: PortraitRequest) {
    let Ok(next_index) = next_variant_index(app, &request) else {
        return;
    };
    let mut enrichment_request = request;
    enrichment_request.variant_index = Some(next_index);
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        match generate_portrait(&app_clone, &enrichment_request).await {
            Ok(response) => {
                log::info!(
                    "[openai] background portrait enrichment {} cat_id={} mood={}",
                    if response.cached {
                        "reused"
                    } else {
                        "generated"
                    },
                    enrichment_request.cat_id,
                    label_for(enrichment_request.mood)
                );
            }
            Err(error) => {
                log::warn!(
                    "[openai] background portrait enrichment failed cat_id={} mood={}: {error}",
                    enrichment_request.cat_id,
                    label_for(enrichment_request.mood)
                );
            }
        }
    });
}

/// Regenerate the portrait for the cat's *current* persisted state. The
/// frontend calls this after `apply_task_outcome` reports `regen_portrait`
/// so it never has to know the level→tier formula or assemble a
/// `PortraitRequest` — all derivation lives here, in Rust.
#[tauri::command]
pub async fn regen_cat_portrait(app: AppHandle) -> Result<PortraitResponse, String> {
    let Some(cat) = store::read_cat(&app).map_err(|e| e.to_string())? else {
        return Err("no cat found — finish onboarding first".into());
    };
    let request = PortraitRequest {
        cat_id: cat.id.clone(),
        cat_type: cat.cat_type,
        mood: cat.mood,
        independence_tier: IndependenceTier::from_level(cat.independence_level),
        accessory_set_hash: "v1".into(),
        variant_index: None,
        skills: cat.skills.clone(),
    };
    let response = generate_portrait(&app, &request)
        .await
        .map_err(|e| e.to_string())?;
    // Persist the freshly-generated path so the dashboard picks it up next
    // time it reads cat state. Flip `portrait_is_base` since we just got
    // back a gpt-image-2 output that needs bg removal in the frontend.
    let mut updated = cat;
    updated.portrait_path = Some(response.path.clone());
    updated.portrait_is_base = response.background_removed;
    store::write_cat(&app, &updated).map_err(|e| e.to_string())?;
    if response.cached {
        queue_catalogue_enrichment(&app, request);
    }
    Ok(response)
}

/// Pre-generate the two portraits the user *might* land on after this
/// interruption (Completed-mood and Dismissed-mood) in parallel, so the
/// post-click `regen_cat_portrait` call can short-circuit on a fresh cache
/// hit and swap instantly. Inaccessible doesn't change mood, so we don't
/// pre-gen for it. Fire-and-forget — Frontend invokes this whenever a new
/// task bundle is shown (initial + every reroll).
#[tauri::command]
pub fn predict_outcome_portraits<R: Runtime>(
    app: AppHandle<R>,
    cat_id: String,
    task_category: TaskCategory,
) -> Result<(), String> {
    let Some(cat) = store::read_cat(&app).map_err(|e| e.to_string())? else {
        return Ok(()); // no cat yet, nothing to predict
    };
    if cat.id != cat_id {
        // Cat changed under us; bail rather than gen for the wrong character.
        return Ok(());
    }
    let history = store::read_task_events(&app).unwrap_or_default();
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        run_outcome_predictions(&app_clone, cat, task_category, history).await;
    });
    Ok(())
}

async fn run_outcome_predictions<R: Runtime>(
    app: &AppHandle<R>,
    cat: Cat,
    task_category: TaskCategory,
    history: Vec<TaskEvent>,
) {
    log::info!(
        "[openai] pre-gen kicked off for cat_id={} (Completed + Dismissed in parallel)",
        cat.id
    );
    let cat_for_completed = cat.clone();
    let cat_for_dismissed = cat;
    let history_completed = history.clone();
    let history_dismissed = history;

    let app_c = app.clone();
    let app_d = app.clone();
    let (c_result, d_result) = tokio::join!(
        predict_one_outcome(
            &app_c,
            cat_for_completed,
            task_category,
            crate::cat_state::TaskOutcome::Completed,
            history_completed,
        ),
        predict_one_outcome(
            &app_d,
            cat_for_dismissed,
            task_category,
            crate::cat_state::TaskOutcome::Dismissed,
            history_dismissed,
        ),
    );
    log::info!(
        "[openai] pre-gen finished — completed: {}, dismissed: {}",
        result_summary(&c_result),
        result_summary(&d_result)
    );
}

async fn predict_one_outcome<R: Runtime>(
    app: &AppHandle<R>,
    mut cat: Cat,
    category: TaskCategory,
    outcome: crate::cat_state::TaskOutcome,
    history: Vec<TaskEvent>,
) -> Result<PortraitResponse> {
    // Mutate the cloned cat as `apply_task_outcome` would in real flow,
    // so the resulting `mood` matches what `derive_mood` will produce on
    // the actual click. Skip the persist — this is a prediction.
    let _ = crate::cat_state::apply_task_outcome(&mut cat, category, outcome, None, &history);
    let request = PortraitRequest {
        cat_id: cat.id.clone(),
        cat_type: cat.cat_type,
        mood: cat.mood,
        independence_tier: IndependenceTier::from_level(cat.independence_level),
        accessory_set_hash: "v1".into(),
        variant_index: None,
        skills: cat.skills.clone(),
    };
    generate_portrait(app, &request).await
}

fn result_summary(result: &Result<PortraitResponse>) -> String {
    match result {
        Ok(response) => {
            if response.cached {
                "cached".to_owned()
            } else {
                "generated".to_owned()
            }
        }
        Err(error) => format!("ERROR: {error}"),
    }
}
