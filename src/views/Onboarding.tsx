import clsx from "clsx";
import { useState } from "react";
import { match } from "ts-pattern";
import { ErrorModal } from "../components/ErrorModal";
import { getSettings, saveSettings, saveUserProfile } from "../lib/api";
import type {
	CatTone,
	Environment,
	Mobility,
	StuckPattern,
	TaskBoundary,
	UserProfile,
} from "../lib/types";
import { newId, nowIso } from "../lib/util";
import { useViewStore } from "../lib/viewStore";

type Step =
	| "welcome"
	| "goals"
	| "stuck"
	| "mobility"
	| "environment"
	| "tone"
	| "boundaries"
	| "intensity"
	| "key"
	| "done";

const stepOrder: Step[] = [
	"welcome",
	"goals",
	"stuck",
	"mobility",
	"environment",
	"tone",
	"boundaries",
	"intensity",
	"key",
	"done",
];

interface DraftProfile {
	goals: string[];
	stuck_patterns: StuckPattern[];
	preferred_tone: CatTone;
	mobility_constraints: Mobility;
	environment_constraints: Environment[];
	task_boundaries: TaskBoundary[];
	interruption_intensity: number;
	api_key: string;
	goals_notes: string;
	stuck_patterns_notes: string;
	tone_notes: string;
	mobility_notes: string;
	environment_notes: string;
	task_boundaries_notes: string;
}

const SUGGESTED_GOALS = [
	"Get unstuck more easily",
	"Doomscroll less",
	"Move my body during the day",
	"Drink more water",
	"Tidy my space",
	"Start tasks I've been avoiding",
];

const STUCK_OPTIONS: { value: StuckPattern; label: string }[] = [
	{ value: "doomscroll", label: "Doomscrolling" },
	{ value: "paralysis", label: "Paralysis / can't start" },
	{ value: "isolation", label: "Isolating from people" },
	{ value: "avoidance", label: "Avoiding hard tasks" },
	{ value: "overwhelm", label: "Overwhelm shutdown" },
];

const MOBILITY_OPTIONS: { value: Mobility; label: string; sub: string }[] = [
	{ value: "low", label: "Low", sub: "Seated tasks only" },
	{ value: "light", label: "Light", sub: "Short stand-ups OK" },
	{ value: "moderate", label: "Moderate", sub: "Short walks OK" },
	{ value: "high", label: "High", sub: "Anything reasonable" },
];

const ENV_OPTIONS: { value: Environment; label: string }[] = [
	{ value: "desk", label: "Desk at home" },
	{ value: "bedroom", label: "Bedroom" },
	{ value: "office", label: "Office (people around)" },
	{ value: "public", label: "Public space" },
	{ value: "shared", label: "Shared space" },
];

const TONE_OPTIONS: { value: CatTone; label: string; sub: string }[] = [
	{ value: "gentle", label: "Gentle", sub: "Soft, patient, warm" },
	{ value: "sassy", label: "Sassy", sub: "Lightly teasing" },
	{ value: "dramatic", label: "Dramatic", sub: "Big feelings" },
	{ value: "chaotic", label: "Chaotic", sub: "Unpredictable, loyal" },
	{ value: "unknown", label: "Surprise me", sub: "Let the cat decide" },
];

const BOUNDARY_OPTIONS: { value: TaskBoundary; label: string }[] = [
	{ value: "no_food", label: "No food / eating tasks" },
	{ value: "no_loud_movement", label: "No loud movement" },
	{ value: "no_leaving_room", label: "No leaving the room" },
	{ value: "no_outside", label: "No going outside" },
	{
		value: "no_social_embarrassment",
		label: "Nothing socially embarrassing",
	},
];

export function Onboarding() {
	const setView = useViewStore((s) => s.setView);
	const [step, setStep] = useState<Step>("welcome");
	const [draft, setDraft] = useState<DraftProfile>({
		goals: [],
		stuck_patterns: [],
		preferred_tone: "unknown",
		mobility_constraints: "light",
		environment_constraints: ["desk"],
		task_boundaries: [],
		interruption_intensity: 2,
		api_key: "",
		goals_notes: "",
		stuck_patterns_notes: "",
		tone_notes: "",
		mobility_notes: "",
		environment_notes: "",
		task_boundaries_notes: "",
	});
	const [saving, setSaving] = useState(false);
	const [error, setError] = useState<string | null>(null);

	const stepIndex = stepOrder.indexOf(step);
	const goNext = () => setStep(stepOrder[stepIndex + 1] ?? "done");
	const goBack = () =>
		setStep(stepOrder[Math.max(stepIndex - 1, 0)] ?? "welcome");

	const toggleArray = <T,>(arr: T[], value: T): T[] =>
		arr.includes(value) ? arr.filter((v) => v !== value) : [...arr, value];

	const finish = async () => {
		setSaving(true);
		setError(null);
		try {
			const profile: UserProfile = {
				id: newId("usr"),
				created_at: nowIso(),
				goals: draft.goals,
				stuck_patterns: draft.stuck_patterns,
				preferred_tone: draft.preferred_tone,
				mobility_constraints: draft.mobility_constraints,
				environment_constraints: draft.environment_constraints,
				task_boundaries: draft.task_boundaries,
				interruption_intensity: draft.interruption_intensity,
				ai_enabled: true,
				goals_notes: draft.goals_notes.trim(),
				stuck_patterns_notes: draft.stuck_patterns_notes.trim(),
				tone_notes: draft.tone_notes.trim(),
				mobility_notes: draft.mobility_notes.trim(),
				environment_notes: draft.environment_notes.trim(),
				task_boundaries_notes: draft.task_boundaries_notes.trim(),
			};
			await saveUserProfile(profile);
			const settings = await getSettings();
			await saveSettings({
				...settings,
				openai_api_key: draft.api_key.trim() || null,
				onboarding_complete: true,
			});
			setView("adopt");
		} catch (e) {
			setError(e instanceof Error ? e.message : String(e));
		} finally {
			setSaving(false);
		}
	};

	const body = match(step)
		.with("welcome", () => (
			<>
				<img
					src="/cat_logo.svg"
					alt="Paws The Scroll"
					className="onboarding-logo"
				/>
				<h1>You found a cat.</h1>
				<p className="lede">
					Or — a cat found you. They're a little needy. They get bored when
					you've been on the rectangle too long, and they have very firm
					opinions about how you should care for them.
				</p>
				<p className="lede">
					This app uses tiny acts of caring for the cat as bridges to tiny acts
					of caring for yourself. None of it is therapy or diagnosis.
				</p>
				<CrisisNote />
				<button type="button" className="primary" onClick={goNext}>
					Continue
				</button>
			</>
		))
		.with("goals", () => (
			<>
				<h2>What would feel good to do more of?</h2>
				<p className="muted">Pick any that fit. You can change these later.</p>
				<div className="chip-row">
					{SUGGESTED_GOALS.map((g) => (
						<Chip
							key={g}
							active={draft.goals.includes(g)}
							onClick={() =>
								setDraft({ ...draft, goals: toggleArray(draft.goals, g) })
							}
						>
							{g}
						</Chip>
					))}
				</div>
				<NotesField
					label="Anything else, in your own words?"
					placeholder="e.g. Spend less of my evening on TikTok and more time stretching."
					value={draft.goals_notes}
					onChange={(v) => setDraft({ ...draft, goals_notes: v })}
				/>
				<StepNav onBack={goBack} onNext={goNext} />
			</>
		))
		.with("stuck", () => (
			<>
				<h2>When overwhelmed, you tend to…</h2>
				<p className="muted">Soft labels. Pick what feels true.</p>
				<div className="chip-row">
					{STUCK_OPTIONS.map((o) => (
						<Chip
							key={o.value}
							active={draft.stuck_patterns.includes(o.value)}
							onClick={() =>
								setDraft({
									...draft,
									stuck_patterns: toggleArray(draft.stuck_patterns, o.value),
								})
							}
						>
							{o.label}
						</Chip>
					))}
				</div>
				<NotesField
					label="Tell me more about how you get stuck"
					placeholder="e.g. I open Reddit during work and lose 90 minutes before I notice."
					value={draft.stuck_patterns_notes}
					onChange={(v) => setDraft({ ...draft, stuck_patterns_notes: v })}
				/>
				<StepNav onBack={goBack} onNext={goNext} />
			</>
		))
		.with("mobility", () => (
			<>
				<h2>How much can your body usually handle?</h2>
				<p className="muted">No judgment. The cat will adapt.</p>
				<div className="card-row">
					{MOBILITY_OPTIONS.map((o) => (
						<RadioCard
							key={o.value}
							active={draft.mobility_constraints === o.value}
							onClick={() =>
								setDraft({ ...draft, mobility_constraints: o.value })
							}
							title={o.label}
							sub={o.sub}
						/>
					))}
				</div>
				<NotesField
					label="Anything specific the cat should know about your body?"
					placeholder="e.g. Sore wrist this week, stairs hurt my knees, prefer no twisting."
					value={draft.mobility_notes}
					onChange={(v) => setDraft({ ...draft, mobility_notes: v })}
				/>
				<StepNav onBack={goBack} onNext={goNext} />
			</>
		))
		.with("environment", () => (
			<>
				<h2>Where will the cat usually find you?</h2>
				<p className="muted">Pick any. Tasks will respect these.</p>
				<div className="chip-row">
					{ENV_OPTIONS.map((o) => (
						<Chip
							key={o.value}
							active={draft.environment_constraints.includes(o.value)}
							onClick={() =>
								setDraft({
									...draft,
									environment_constraints: toggleArray(
										draft.environment_constraints,
										o.value,
									),
								})
							}
						>
							{o.label}
						</Chip>
					))}
				</div>
				<NotesField
					label="Anything else about where you'll be?"
					placeholder="e.g. Roommate sleeps in the next room — keep it quiet after 10pm."
					value={draft.environment_notes}
					onChange={(v) => setDraft({ ...draft, environment_notes: v })}
				/>
				<StepNav onBack={goBack} onNext={goNext} />
			</>
		))
		.with("tone", () => (
			<>
				<h2>What tone of cat sounds nice?</h2>
				<p className="muted">The cat's personality will develop over time.</p>
				<div className="card-row">
					{TONE_OPTIONS.map((o) => (
						<RadioCard
							key={o.value}
							active={draft.preferred_tone === o.value}
							onClick={() => setDraft({ ...draft, preferred_tone: o.value })}
							title={o.label}
							sub={o.sub}
						/>
					))}
				</div>
				<NotesField
					label="Anything the cat should never sound like?"
					placeholder="e.g. No sarcasm, no shame, no productivity-bro energy."
					value={draft.tone_notes}
					onChange={(v) => setDraft({ ...draft, tone_notes: v })}
				/>
				<StepNav onBack={goBack} onNext={goNext} />
			</>
		))
		.with("boundaries", () => (
			<>
				<h2>Anything the cat should never ask?</h2>
				<p className="muted">Hard nos. The cat will respect these.</p>
				<div className="chip-row">
					{BOUNDARY_OPTIONS.map((o) => (
						<Chip
							key={o.value}
							active={draft.task_boundaries.includes(o.value)}
							onClick={() =>
								setDraft({
									...draft,
									task_boundaries: toggleArray(draft.task_boundaries, o.value),
								})
							}
						>
							{o.label}
						</Chip>
					))}
				</div>
				<NotesField
					label="Other limits worth naming?"
					placeholder="e.g. Don't suggest calling family, no journaling prompts, never mention exercise."
					value={draft.task_boundaries_notes}
					onChange={(v) => setDraft({ ...draft, task_boundaries_notes: v })}
				/>
				<StepNav onBack={goBack} onNext={goNext} />
			</>
		))
		.with("intensity", () => (
			<>
				<h2>How strong should the interruptions feel?</h2>
				<p className="muted">
					You can change this anytime. The cat will still be needy either way.
				</p>
				<div className="slider-row">
					<input
						type="range"
						min={1}
						max={3}
						step={1}
						value={draft.interruption_intensity}
						onChange={(e) =>
							setDraft({
								...draft,
								interruption_intensity: Number(e.currentTarget.value),
							})
						}
					/>
					<div className="slider-labels">
						<span>Soft</span>
						<span>Normal</span>
						<span>Hard to ignore</span>
					</div>
				</div>
				<StepNav onBack={goBack} onNext={goNext} />
			</>
		))
		.with("key", () => (
			<>
				<h2>One last thing — your OpenAI key.</h2>
				<p className="muted">
					Stored locally on your machine. The cat uses it to generate dialogue
					and tasks. Without a key, the cat is mute.
				</p>
				<input
					type="password"
					placeholder="sk-..."
					value={draft.api_key}
					onChange={(e) =>
						setDraft({ ...draft, api_key: e.currentTarget.value })
					}
					className="text-input"
				/>
				<CrisisNote />
				<StepNav
					onBack={goBack}
					onNext={finish}
					nextLabel={saving ? "Saving…" : "Finish"}
					nextDisabled={saving}
				/>
			</>
		))
		.with("done", () => null)
		.exhaustive();

	return (
		<div className="onboarding">
			<div className="onboarding-card">{body}</div>
			<ErrorModal
				message={error}
				onDismiss={() => setError(null)}
				title="Couldn't save your setup."
			/>
		</div>
	);
}

function Chip({
	active,
	onClick,
	children,
}: {
	active: boolean;
	onClick: () => void;
	children: React.ReactNode;
}) {
	return (
		<button
			type="button"
			className={clsx("chip", active && "chip-active")}
			onClick={onClick}
		>
			{children}
		</button>
	);
}

function RadioCard({
	active,
	onClick,
	title,
	sub,
}: {
	active: boolean;
	onClick: () => void;
	title: string;
	sub: string;
}) {
	return (
		<button
			type="button"
			className={clsx("radio-card", active && "radio-card-active")}
			onClick={onClick}
		>
			<div className="radio-card-title">{title}</div>
			<div className="radio-card-sub">{sub}</div>
		</button>
	);
}

function StepNav({
	onBack,
	onNext,
	nextLabel,
	nextDisabled,
}: {
	onBack: () => void;
	onNext: () => void;
	nextLabel?: string;
	nextDisabled?: boolean;
}) {
	return (
		<div className="step-nav">
			<button type="button" className="ghost" onClick={onBack}>
				Back
			</button>
			<button
				type="button"
				className="primary"
				onClick={onNext}
				disabled={nextDisabled}
			>
				{nextLabel ?? "Continue"}
			</button>
		</div>
	);
}

function NotesField({
	label,
	placeholder,
	value,
	onChange,
}: {
	label: string;
	placeholder: string;
	value: string;
	onChange: (next: string) => void;
}) {
	return (
		<label className="notes-field">
			<span className="notes-field-label">{label}</span>
			<textarea
				className="notes-field-input"
				placeholder={placeholder}
				rows={3}
				value={value}
				onChange={(e) => onChange(e.currentTarget.value)}
			/>
		</label>
	);
}

export function CrisisNote() {
	return (
		<aside className="crisis-note">
			This app is not emergency support or a substitute for professional care.
			If you might hurt yourself or someone else, contact local emergency
			services or a crisis line now.
		</aside>
	);
}
