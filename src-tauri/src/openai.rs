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
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Runtime};

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
    pub time_of_day_label: Option<String>,
    pub reroll_index: u8,
    pub recent_completed_categories: Vec<String>,
    pub recent_dismissed_categories: Vec<String>,
    pub want_fallback: bool,
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

fn http_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_mins(1))
        .build()
        .context("failed to build OpenAI HTTP client")
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
    let tone = match ctx.cat_tone {
        CatTone::Gentle => "gentle, patient, softly affectionate",
        CatTone::Sassy => "sassy, lightly teasing, never cruel",
        CatTone::Dramatic => "theatrical, melodramatic, big feelings about small things",
        CatTone::Chaotic => "chaotic, unpredictable, loyal underneath",
        CatTone::Unknown => "ambiguous; let the cat surprise the user",
    };
    let cat_kind = match ctx.cat_type {
        CatType::OrangeFat => "an orange fat cat — food-motivated, theatrical, emotionally obvious",
        CatType::Void => "a void cat — quiet, mysterious, intense, secretly affectionate",
        CatType::ScrunglyStreet => "a scrungly street cat — chaotic, scrappy, weirdly loyal",
    };
    format!(
        "You write tiny behavioral-activation tasks framed as a cat needing care.\n\
         The cat is {cat_kind}. The cat's tone is {tone}.\n\
         RULES:\n\
         - Task must be doable in under {max_seconds} seconds, indoors, no special items unless the user already has them in a normal home.\n\
         - Never ask the user to leave their room unless they explicitly opted in.\n\
         - Never ask about food/eating if the user has the no_food boundary.\n\
         - Never use clinical language (depression, ADHD, etc.). Never shame.\n\
         - Match mobility level to the user's mobility constraints.\n\
         - Cat line should reference the cat's NEED (hungry/bored/etc.), not lecture the user.\n\
         - Keep titles short (≤6 words). Keep instructions to 1-2 sentences.\n\
         - The cat is needy, but never abandons or punishes the user.\n\
         {fallback_hint}",
        max_seconds = if ctx.want_fallback { 60 } else { 240 },
        fallback_hint = if ctx.want_fallback {
            "- FALLBACK MODE: pick the easiest possible task. No items, no unusual movement, no embarrassment. Mark `fallback_safe: true` and `mobility_level: \"light\"`."
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
    if let Some(t) = &ctx.time_of_day_label {
        lines.push(format!("Time of day: {t}"));
    }
    if !ctx.recent_completed_categories.is_empty() {
        lines.push(format!(
            "Recently completed categories: {}",
            ctx.recent_completed_categories.join(", ")
        ));
    }
    if !ctx.recent_dismissed_categories.is_empty() {
        lines.push(format!(
            "Recently dismissed categories: {}",
            ctx.recent_dismissed_categories.join(", ")
        ));
    }
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

/// Tauri event payload broadcast on each partial frame from the streaming
/// image generation. The frontend swaps the placeholder for `data_url` as
/// each partial lands; the final frame is also written to disk by the
/// caller and read back via `read_portrait_bytes`.
pub const PORTRAIT_PROGRESS_EVENT: &str = "cat-portrait-progress";

#[derive(Clone, Debug, Serialize)]
struct PortraitProgress {
    cat_id: String,
    partial_index: Option<u64>,
    is_final: bool,
    data_url: String,
}

/// Streamed image *edit* against a hand-drawn base portrait. Switching from
/// `images/generations` to `images/edits` lets us anchor the visual style to
/// `assets/{breed}.png` so we don't have to re-describe the style in every
/// prompt. Multipart payload because the edits endpoint takes the source
/// image as a file part. `partial_images: 3` still streams partial frames
/// over SSE, costing ~300 extra image-output tokens per cat.
async fn call_image_edit_streaming<R: Runtime>(
    app: &AppHandle<R>,
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
        .text("stream", "true")
        .text("partial_images", "3")
        .text("n", "1")
        .part(
            "image",
            reqwest::multipart::Part::bytes(base_image.to_vec())
                .file_name("base.png")
                .mime_str("image/png")
                .context("failed to set image part mime")?,
        );
    let resp = client
        .post(format!("{OPENAI_BASE_URL}/images/edits"))
        .bearer_auth(api_key)
        .header("Accept", "text/event-stream")
        .multipart(form)
        .send()
        .await
        .context("OpenAI image edit request failed")?;
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        bail!("OpenAI images returned {status}: {text}");
    }

    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();
    let mut last_b64: Option<String> = None;
    let mut event_count: u32 = 0;
    let mut partial_count: u32 = 0;

    log::info!("[openai] image stream started for cat_id={cat_id}");

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("failed to read SSE chunk")?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(boundary) = buffer.find("\n\n") {
            let event_block = buffer[..boundary].to_owned();
            buffer.drain(..boundary + 2);
            event_count += 1;
            let emitted = handle_image_sse_event(app, cat_id, &event_block, &mut last_b64);
            if emitted {
                partial_count += 1;
            }
        }
    }
    if !buffer.is_empty() {
        event_count += 1;
        let emitted = handle_image_sse_event(app, cat_id, &buffer, &mut last_b64);
        if emitted {
            partial_count += 1;
        }
    }

    log::info!(
        "[openai] image stream finished for cat_id={cat_id}: events={event_count} partials={partial_count}"
    );

    let final_b64 =
        last_b64.ok_or_else(|| anyhow!("OpenAI image stream ended without any frames"))?;

    // Re-emit the last frame as the final so the frontend has a clean signal
    // to swap from "still streaming" to "done".
    let _ = app.emit(
        PORTRAIT_PROGRESS_EVENT,
        PortraitProgress {
            cat_id: cat_id.to_owned(),
            partial_index: None,
            is_final: true,
            data_url: format!("data:image/jpeg;base64,{final_b64}"),
        },
    );

    base64::engine::general_purpose::STANDARD
        .decode(final_b64.as_bytes())
        .context("failed to base64-decode final streamed image")
}

/// Returns `true` if at least one b64 frame was extracted and emitted.
fn handle_image_sse_event<R: Runtime>(
    app: &AppHandle<R>,
    cat_id: &str,
    event_block: &str,
    last_b64: &mut Option<String>,
) -> bool {
    let mut event_type: Option<String> = None;
    let mut emitted = false;
    for line in event_block.lines() {
        if let Some(rest) = line
            .strip_prefix("event: ")
            .or_else(|| line.strip_prefix("event:"))
        {
            event_type = Some(rest.trim().to_owned());
            continue;
        }
        let Some(payload) = line
            .strip_prefix("data: ")
            .or_else(|| line.strip_prefix("data:"))
        else {
            continue;
        };
        let payload = payload.trim();
        if payload.is_empty() || payload == "[DONE]" {
            continue;
        }
        let json: serde_json::Value = match serde_json::from_str(payload) {
            Ok(v) => v,
            Err(error) => {
                log::warn!("[openai] failed to parse SSE data: {error} payload={payload}");
                continue;
            }
        };
        // OpenAI image streaming has shipped two payload shapes in the wild:
        // the b64 may sit at the top level, under `data` (singular), or under
        // `data[0]` (array). Try each.
        let b64_value = json
            .get("b64_json")
            .or_else(|| json.pointer("/data/b64_json"))
            .or_else(|| json.pointer("/data/0/b64_json"));
        let Some(b64) = b64_value.and_then(|v| v.as_str()) else {
            log::info!(
                "[openai] SSE event without b64 (event={:?}): {}",
                event_type,
                truncate_for_log(payload)
            );
            continue;
        };
        let partial_index = json
            .get("partial_image_index")
            .or_else(|| json.pointer("/data/partial_image_index"))
            .and_then(serde_json::Value::as_u64);
        log::info!(
            "[openai] streaming partial: event={:?} index={:?} bytes_b64={}",
            event_type,
            partial_index,
            b64.len()
        );
        let _ = app.emit(
            PORTRAIT_PROGRESS_EVENT,
            PortraitProgress {
                cat_id: cat_id.to_owned(),
                partial_index,
                is_final: false,
                data_url: format!("data:image/jpeg;base64,{b64}"),
            },
        );
        *last_b64 = Some(b64.to_owned());
        emitted = true;
    }
    emitted
}

fn truncate_for_log(s: &str) -> String {
    const MAX: usize = 240;
    if s.len() <= MAX {
        s.to_owned()
    } else {
        format!("{}…(+{} bytes)", &s[..MAX], s.len() - MAX)
    }
}

/// Build an *edit* prompt — short, because the base image already carries the
/// breed and the visual style. We only describe what should *change* from
/// the base: mood, independence, earned-skill details. Keep the same
/// character, same composition, same illustration style.
fn portrait_prompt(req: &PortraitRequest) -> String {
    let mood = match req.mood {
        CatMood::Content => "calm, content expression",
        CatMood::Smug => "smug, eyes-half-closed satisfaction",
        CatMood::Sulky => "sulky, ears slightly back, dramatic disappointment",
        CatMood::Excited => "alert, ears forward, bright eyes",
        CatMood::Dramatic => "exaggerated theatrical pose, big eyes",
        CatMood::Sleepy => "loafing, sleepy, blinking softly",
        CatMood::Affectionate => "cheek-rubbing, warm, eyes closed in trust",
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
        "Same cat, same character, same composition. {PORTRAIT_STYLE_ANCHOR} \
         Adjust pose and expression: {mood}; {independence}.{skill_clause} \
         Centered, full body, no text."
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

    if image_cache::read_cached(&path).is_some() {
        return Ok(PortraitResponse {
            path: path.to_string_lossy().into_owned(),
            cached: true,
        });
    }
    let api_key = require_key(app)?;
    let prompt = portrait_prompt(req);
    let base_image = cat_bases::bytes_for(req.cat_type);
    let bytes = call_image_edit_streaming(app, &req.cat_id, &api_key, &prompt, base_image).await?;
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

#[tauri::command]
pub async fn read_portrait_bytes(path: String) -> Result<String, String> {
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
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
    // time it reads cat state.
    let mut updated = cat;
    updated.portrait_path = Some(response.path.clone());
    store::write_cat(&app, &updated).map_err(|e| e.to_string())?;
    Ok(response)
}
