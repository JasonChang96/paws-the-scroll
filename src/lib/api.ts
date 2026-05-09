// Typed bridges to Rust `#[tauri::command]` functions. Keep names + payload
// shapes in sync with src-tauri/src/{store,openai}.rs.

import { invoke } from "@tauri-apps/api/core";
import type {
	ActivityAggregate,
	AppCategory,
	Cat,
	CatMood,
	GeneratedTaskBundle,
	IndependenceTier,
	Settings,
	SkillId,
	TaskCategory,
	TaskEvent,
	TimeOfDay,
	UserProfile,
} from "./types";

export const getUserProfile = (): Promise<UserProfile | null> =>
	invoke("get_user_profile");

export const saveUserProfile = (profile: UserProfile): Promise<void> =>
	invoke("save_user_profile", { profile });

export const getCat = (): Promise<Cat | null> => invoke("get_cat");

export const saveCat = (cat: Cat): Promise<void> => invoke("save_cat", { cat });

export const getSettings = (): Promise<Settings> => invoke("get_settings");

export const saveSettings = (settings: Settings): Promise<void> =>
	invoke("save_settings", { settings });

export const recordTaskEvent = (event: TaskEvent): Promise<void> =>
	invoke("record_task_event", { event });

export const listTaskEvents = (): Promise<TaskEvent[]> =>
	invoke("list_task_events");

export const listAggregates = (): Promise<ActivityAggregate[]> =>
	invoke("list_aggregates");

/// Wipes the persisted store, the on-disk sprite cache, and emits
/// `cat-updated` so listeners refresh. The frontend should send the user
/// back to onboarding after this resolves.
export const factoryReset = (): Promise<void> => invoke("factory_reset");

export interface InterruptionTaskContext {
	goals: string[];
	stuck_patterns: UserProfile["stuck_patterns"];
	mobility: UserProfile["mobility_constraints"];
	environment: UserProfile["environment_constraints"];
	task_boundaries: UserProfile["task_boundaries"];
	cat_type: Cat["type"];
	cat_tone: UserProfile["preferred_tone"];
	cat_mood: Cat["mood"];
	cat_visible_traits: string[];
	cat_hidden_traits: string[];
	current_active_app: string | null;
	current_active_app_category: AppCategory | null;
	current_window_title: string | null;
	current_browser_url: string | null;
	time_of_day_label: TimeOfDay | null;
	reroll_index: number;
	recent_completed_categories: string[];
	recent_dismissed_categories: string[];
	want_fallback: boolean;
	goals_notes: string;
	stuck_patterns_notes: string;
	tone_notes: string;
	mobility_notes: string;
	environment_notes: string;
	task_boundaries_notes: string;
	active_streak_seconds: number;
	today_active_seconds: number;
	today_social_seconds: number;
	today_interruptions: number;
	today_completed: number;
	today_dismissed: number;
}

export const generateInterruptionTask = (
	context: InterruptionTaskContext,
): Promise<GeneratedTaskBundle> =>
	invoke("generate_interruption_task", { context });

export interface PortraitRequest {
	cat_id: string;
	cat_type: Cat["type"];
	mood: CatMood;
	independence_tier: IndependenceTier;
	accessory_set_hash: string;
	skills: SkillId[];
}

export type TaskOutcome = "completed" | "dismissed" | "inaccessible";

export interface OutcomePayload {
	category: TaskCategory;
	outcome: TaskOutcome;
	completed_at: string;
}

export interface OutcomeEffect {
	regen_portrait: boolean;
	unlocked_skills: SkillId[];
	streak_days: number;
	previous_portrait_signature: string;
	current_portrait_signature: string;
}

export interface ApplyTaskOutcomeResponse {
	cat: Cat;
	effect: OutcomeEffect;
}

export const applyTaskOutcome = (
	payload: OutcomePayload,
	lastEvent: TaskEvent | null,
): Promise<ApplyTaskOutcomeResponse> =>
	invoke("apply_task_outcome", { payload, lastEvent });

export const listCatSkills = (): Promise<SkillId[]> =>
	invoke("list_cat_skills");

export interface PortraitResponse {
	path: string;
	cached: boolean;
}

export const generateCatPortrait = (
	request: PortraitRequest,
): Promise<PortraitResponse> => invoke("generate_cat_portrait", { request });

/// Regenerate the portrait for the cat's *current* persisted state. Use this
/// instead of `generateCatPortrait` when reacting to cat-state evolution —
/// Rust derives the tier and assembles the request so callers never duplicate
/// the level→tier formula.
export const regenCatPortrait = (): Promise<PortraitResponse> =>
	invoke("regen_cat_portrait");

/// Seed the cat's initial portrait at adoption time. No OpenAI call — Rust
/// just writes the embedded base PNG to the sprite cache so subsequent
/// reads behave the same as a generated portrait. Future state-change
/// regenerations go through `regenCatPortrait` and hit the edit endpoint.
export const seedInitialPortrait = (
	catId: string,
	catType: Cat["type"],
): Promise<PortraitResponse> =>
	invoke("seed_initial_portrait", { catId, catType });

export const readPortraitBytes = (path: string): Promise<string> =>
	invoke("read_portrait_bytes", { path });
