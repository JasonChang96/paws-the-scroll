//! `OpenAI` client for cat dialogue, behavioral-activation tasks, completion
//! reactions, non-use rewards, and cat sprite generation.
//!
//! All traffic flows through Rust so the API key never enters the webview.
//! The frontend passes a `TaskContext` (user profile slice + cat state +
//! environment hints + reroll counter) and gets a fully-validated
//! `GeneratedTaskBundle` back, or a soft error on guardrail failure.

#![allow(dead_code)]

use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime};

use crate::cat_bases;
use crate::cat_state;
use crate::image_cache;
use crate::model::{
    CatMood, CatTone, CatType, Environment, GeneratedTaskBundle, IndependenceTier, Mobility,
    SkillId, StuckPattern, TaskBoundary,
};
use crate::store;

const OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
const CHAT_MODEL: &str = "gpt-5.5";
const IMAGE_MODEL: &str = "gpt-image-2";

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
    /// Skill IDs the cat has earned. The prompt builder turns these into
    /// visual cues so the cat looks the part.
    #[serde(default)]
    pub skills: Vec<SkillId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortraitResponse {
    pub path: String,
    pub cached: bool,
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
fn task_user_prompt(ctx: &TaskContext) -> String {
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
    let user_prompt = task_user_prompt(ctx);
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

pub async fn generate_portrait<R: Runtime>(
    app: &AppHandle<R>,
    req: &PortraitRequest,
) -> Result<PortraitResponse> {
    let cat_type_label = serde_json::to_value(req.cat_type)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default();
    let mood_label = serde_json::to_value(req.mood)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default();
    let tier_label = serde_json::to_value(req.independence_tier)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default();
    let skills_hash = cat_state::skill_set_hash(&req.skills);
    let key_parts = [
        req.cat_id.as_str(),
        cat_type_label.as_str(),
        mood_label.as_str(),
        tier_label.as_str(),
        req.accessory_set_hash.as_str(),
        skills_hash.as_str(),
    ];
    let key = image_cache::make_key(&key_parts);
    let path = image_cache::path_for_key(app, &key)?;

    // No cache hit short-circuit. Every regen call hits gpt-image-2 so the
    // user actually sees a fresh image each time, even when mood + tier +
    // skills repeat. The on-disk file is still the canonical place we
    // store the most-recent portrait — we just always overwrite it.
    log::info!(
        "[openai] regenerating portrait cat_id={} mood={mood_label} tier={tier_label} skills={skills_hash}",
        req.cat_id
    );
    let api_key = require_key(app)?;
    let prompt = portrait_prompt(req);
    let base_image = cat_bases::bytes_for(req.cat_type);
    let _ = app;
    let bytes = call_image_edit(&req.cat_id, &api_key, &prompt, base_image).await?;
    image_cache::write_cached(&path, &bytes)?;
    Ok(PortraitResponse {
        path: path.to_string_lossy().into_owned(),
        cached: false,
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
    let comma = data_url
        .find(',')
        .ok_or_else(|| "input is not a data URL".to_string())?;
    let b64 = &data_url[comma + 1..];
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.as_bytes())
        .map_err(|e| e.to_string())?;
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    if let Ok(Some(mut cat)) = store::read_cat(&app) {
        if cat.portrait_path.as_deref() == Some(path.as_str()) && !cat.portrait_is_base {
            cat.portrait_is_base = true;
            store::write_cat(&app, &cat).map_err(|e| e.to_string())?;
        }
    }
    log::info!(
        "[openai] persisted stripped portrait: {} bytes at {path}",
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
        skills: Vec::new(),
    };
    let cat_type_label = serde_json::to_value(request.cat_type)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default();
    let mood_label = serde_json::to_value(request.mood)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default();
    let tier_label = serde_json::to_value(request.independence_tier)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default();
    let skills_hash = cat_state::skill_set_hash(&request.skills);
    let key = image_cache::make_key(&[
        request.cat_id.as_str(),
        cat_type_label.as_str(),
        mood_label.as_str(),
        tier_label.as_str(),
        request.accessory_set_hash.as_str(),
        skills_hash.as_str(),
    ]);
    let path = image_cache::path_for_key(&app, &key).map_err(|e| e.to_string())?;
    if image_cache::read_cached(&path).is_none() {
        image_cache::write_cached(&path, cat_bases::bytes_for(cat_type))
            .map_err(|e| e.to_string())?;
    }
    Ok(PortraitResponse {
        path: path.to_string_lossy().into_owned(),
        cached: false,
    })
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
    updated.portrait_is_base = false;
    store::write_cat(&app, &updated).map_err(|e| e.to_string())?;
    Ok(response)
}
