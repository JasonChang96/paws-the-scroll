// Mirrors src-tauri/src/model.rs. Keep field names and string variants in
// sync; serde uses snake_case rename_all on every enum.

export type CatType = "orange_fat" | "void" | "scrungly_street";

export type CatTone = "gentle" | "sassy" | "dramatic" | "chaotic" | "unknown";

export type StuckPattern =
	| "doomscroll"
	| "paralysis"
	| "isolation"
	| "avoidance"
	| "overwhelm";

export type Mobility = "low" | "light" | "moderate" | "high";

export type Environment = "desk" | "bedroom" | "office" | "public" | "shared";

export type TaskBoundary =
	| "no_food"
	| "no_loud_movement"
	| "no_leaving_room"
	| "no_outside"
	| "no_social_embarrassment";

export type CatNeedKind =
	| "hungry"
	| "bored"
	| "lonely"
	| "dirty_litter"
	| "play"
	| "attention"
	| "dramatic"
	| "cursed_find";

export type TaskCategory =
	| "movement"
	| "hydration"
	| "environment"
	| "food"
	| "stretching"
	| "grounding"
	| "task_init";

export type CatMood =
	| "content"
	| "smug"
	| "sulky"
	| "excited"
	| "dramatic"
	| "sleepy"
	| "affectionate";

/// Bucketed independence level. Mirrors `IndependenceTier` in
/// src-tauri/src/model.rs — Rust owns the level→tier formula.
/// Note: serde's snake_case treats digits as part of the prior word, so
/// `Tier0` serializes as `tier0`, not `tier_0`.
export type IndependenceTier = "tier0" | "tier1" | "tier2" | "tier3";

/// Cat skills earned through streak milestones. Mirrors `SkillId` in
/// src-tauri/src/model.rs. Each skill has a passive-decay rule and a visual
/// cue in the portrait — adding one means updating both sides.
export type SkillId =
	| "occasional_self_feeding"
	| "independent_play"
	| "self_grooming";

export type TaskSource = "ai" | "fallback" | "demo_trigger";

export type AppCategory = "social" | "browser" | "other";

export type InterruptionSource = "scheduler" | "demo_trigger";

export type TimeOfDay = "morning" | "afternoon" | "evening" | "late_night";

export type OverlayMode = "companion" | "interruption";

export interface UserProfile {
	id: string;
	created_at: string;
	goals: string[];
	stuck_patterns: StuckPattern[];
	preferred_tone: CatTone;
	mobility_constraints: Mobility;
	environment_constraints: Environment[];
	task_boundaries: TaskBoundary[];
	interruption_intensity: number;
	ai_enabled: boolean;
	/// Free-form per-step notes paired with the chip/card picks. Always
	/// present (empty string when the user wrote nothing); old profiles
	/// missing the field deserialize as "" via serde default on the Rust
	/// side.
	goals_notes: string;
	stuck_patterns_notes: string;
	tone_notes: string;
	mobility_notes: string;
	environment_notes: string;
	task_boundaries_notes: string;
}

export interface CatNeeds {
	hunger: number;
	boredom: number;
	loneliness: number;
	dirty_litter: number;
	play_drive: number;
	attention: number;
}

export interface CatItem {
	id: string;
	name: string;
	description: string;
	acquired_at: string;
}

export interface StoryEvent {
	id: string;
	at: string;
	text: string;
}

export interface Cat {
	id: string;
	type: CatType;
	name: string;
	visible_traits: string[];
	hidden_traits: string[];
	needs: CatNeeds;
	mood: CatMood;
	independence_level: number;
	skills: SkillId[];
	items: CatItem[];
	story_events: StoryEvent[];
	portrait_path: string | null;
}

export interface ActivityAggregate {
	date: string;
	active_seconds: number;
	idle_seconds: number;
	social_seconds: number;
	focus_seconds: number;
	interruptions: number;
	tasks_completed: number;
	rerolls: number;
	dismissals: number;
	time_away_after_interruptions_seconds: number;
}

export interface TaskEvent {
	id: string;
	created_at: string;
	source: TaskSource;
	category: TaskCategory;
	difficulty: number;
	app_category: string | null;
	reroll_index: number;
	completed: boolean;
	dismissed: boolean;
	marked_inaccessible: boolean;
}

export interface Settings {
	openai_api_key: string | null;
	demo_mode: boolean;
	grace_period_seconds: number;
	idle_threshold_seconds: number;
	interruption_window_min_seconds: number;
	interruption_window_max_seconds: number;
	social_apps_extra: string[];
	onboarding_complete: boolean;
}

export interface GeneratedTask {
	title: string;
	instruction: string;
	category: TaskCategory;
	difficulty: number;
	estimated_seconds: number;
	requires_items: boolean;
	requires_leaving_room: boolean;
	mobility_level: Mobility;
	fallback_safe: boolean;
}

export interface ForegroundApp {
	bundle_id: string | null;
	display_name: string | null;
	process_id: number | null;
	/// Window title from macOS Accessibility API. Null when access not
	/// granted or the app doesn't expose AXTitle.
	window_title: string | null;
	/// Active tab/document URL for browsers (AXDocument). Same null semantics.
	browser_url: string | null;
}

export interface GeneratedTaskBundle {
	cat_line: string;
	need: CatNeedKind;
	task: GeneratedTask;
	completion_line: string;
	safety_notes: string[];
}
