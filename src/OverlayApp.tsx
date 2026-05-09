import { type Event, emit, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import clsx from "clsx";
import { useCallback, useEffect, useRef, useState } from "react";
import { match } from "ts-pattern";
import beanPng from "../assets/bean.png";
import mangoPng from "../assets/mango.png";
import plutoPng from "../assets/pluto.png";
import {
	applyTaskOutcome,
	generateInterruptionTask,
	getCat,
	getUserProfile,
	readPortraitBytes,
	recordTaskEvent,
	regenCatPortrait,
	type TaskOutcome,
} from "./lib/api";
import { stripBackground } from "./lib/backgroundRemoval";
import {
	enterInterruption,
	exitInterruption,
	type InterruptionPayload,
	onInterruptionRequested,
} from "./lib/overlayApi";
import type { Cat, GeneratedTaskBundle, UserProfile } from "./lib/types";
import { newId, nowIso } from "./lib/util";

/// Static base PNG fallback shown only when nothing has been bg-removed yet
/// (cold start, or first cat after adoption). After the first successful
/// strip we never reveal an un-stripped portrait again.
function baseImageFor(catType: Cat["type"]): string {
	switch (catType) {
		case "orange_fat":
			return mangoPng;
		case "void":
			return plutoPng;
		case "scrungly_street":
			return beanPng;
	}
}

type Mode =
	| { kind: "companion" }
	| { kind: "loading"; rerollIndex: number; payload: InterruptionPayload }
	| {
			kind: "task";
			rerollIndex: number;
			bundle: GeneratedTaskBundle;
			payload: InterruptionPayload;
			disabledUntil: number;
	  }
	| { kind: "error"; message: string };

const TASK_READY_EVENT = "paws-task-ready";
const TASK_RESET_EVENT = "paws-task-reset";

interface TaskReadyPayload {
	rerollIndex: number;
	bundle: GeneratedTaskBundle;
	payload: InterruptionPayload;
	disabledUntil: number;
}

function OverlayApp() {
	const window = getCurrentWindow();
	const isPrimary = window.label === "overlay";
	const [cat, setCat] = useState<Cat | null>(null);
	const [profile, setProfile] = useState<UserProfile | null>(null);
	const [portraitDataUrl, setPortraitDataUrl] = useState<string | null>(null);
	const [mode, setMode] = useState<Mode>({ kind: "companion" });
	const [tick, setTick] = useState(0);
	const generationLockRef = useRef(false);
	const completedCategoriesRef = useRef<string[]>([]);
	const dismissedCategoriesRef = useRef<string[]>([]);

	useEffect(() => {
		const interval = setInterval(() => setTick((t) => t + 1), 250);
		return () => clearInterval(interval);
	}, []);

	// Track the currently-displayed portrait so the async refresh can
	// decide whether to show a fallback. We don't read state directly inside
	// async callbacks because the closure would capture a stale value.
	const currentPortraitRef = useRef<string | null>(null);
	useEffect(() => {
		currentPortraitRef.current = portraitDataUrl;
	}, [portraitDataUrl]);

	const refreshCat = useCallback(async () => {
		const c = await getCat();
		setCat(c);
		if (!c?.portrait_path) {
			setPortraitDataUrl(null);
			return;
		}
		try {
			const raw = await readPortraitBytes(c.portrait_path);
			// Skip bg removal when the portrait is still an embedded base
			// PNG — those ship with transparent backgrounds already, so
			// running the model is wasted work (and a 1-3s flash of the
			// fallback for no reason).
			if (c.portrait_is_base) {
				setPortraitDataUrl(raw);
				return;
			}
			// Otherwise: don't reveal the opaque cat. While the strip runs,
			// keep the previous bg-removed portrait on screen, or fall back
			// to the breed's static base PNG on cold start.
			if (currentPortraitRef.current === null) {
				setPortraitDataUrl(baseImageFor(c.type));
			}
			setPortraitDataUrl(await stripBackground(raw));
		} catch {
			setPortraitDataUrl(null);
		}
	}, []);

	useEffect(() => {
		(async () => {
			const p = await getUserProfile();
			setProfile(p);
			await refreshCat();
		})();
	}, [refreshCat]);

	// Refresh whenever Rust persists a new cat (adoption, task outcome,
	// time-away rewards). Fires from `store::write_cat`.
	useEffect(() => {
		let unlisten: (() => void) | undefined;
		(async () => {
			unlisten = await listen("cat-updated", () => {
				void refreshCat();
			});
		})();
		return () => unlisten?.();
	}, [refreshCat]);

	const generateAndAnnounce = useCallback(
		async (rerollIndex: number, payload: InterruptionPayload) => {
			if (!cat || !profile || generationLockRef.current) return;
			generationLockRef.current = true;
			try {
				const bundle = await generateInterruptionTask({
					goals: profile.goals,
					stuck_patterns: profile.stuck_patterns,
					mobility: profile.mobility_constraints,
					environment: profile.environment_constraints,
					task_boundaries: profile.task_boundaries,
					cat_type: cat.type,
					cat_tone: profile.preferred_tone,
					cat_mood: cat.mood,
					cat_visible_traits: cat.visible_traits,
					cat_hidden_traits: cat.hidden_traits,
					current_active_app: payload.active_app?.display_name ?? null,
					current_active_app_category: payload.active_app_category,
					current_window_title: payload.active_app?.window_title ?? null,
					current_browser_url: payload.active_app?.browser_url ?? null,
					time_of_day_label: payload.time_of_day_label,
					reroll_index: rerollIndex,
					recent_completed_categories: completedCategoriesRef.current,
					recent_dismissed_categories: dismissedCategoriesRef.current,
					want_fallback: rerollIndex >= 5,
					goals_notes: profile.goals_notes ?? "",
					stuck_patterns_notes: profile.stuck_patterns_notes ?? "",
					tone_notes: profile.tone_notes ?? "",
					mobility_notes: profile.mobility_notes ?? "",
					environment_notes: profile.environment_notes ?? "",
					task_boundaries_notes: profile.task_boundaries_notes ?? "",
					active_streak_seconds: payload.active_streak_seconds,
					today_active_seconds: payload.today_active_seconds,
					today_social_seconds: payload.today_social_seconds,
					today_interruptions: payload.today_interruptions,
					today_completed: payload.today_completed,
					today_dismissed: payload.today_dismissed,
				});
				const ready: TaskReadyPayload = {
					rerollIndex,
					bundle,
					payload,
					disabledUntil: Date.now() + 5_000,
				};
				await emit(TASK_READY_EVENT, ready);
			} catch (e) {
				const message = e instanceof Error ? e.message : String(e);
				setMode({ kind: "error", message });
			} finally {
				generationLockRef.current = false;
			}
		},
		[cat, profile],
	);

	useEffect(() => {
		if (!isPrimary) return;
		let unlisten: (() => void) | undefined;
		(async () => {
			unlisten = await onInterruptionRequested(async (payload) => {
				await enterInterruption();
				setMode({ kind: "loading", rerollIndex: 0, payload });
				await generateAndAnnounce(0, payload);
			});
		})();
		return () => unlisten?.();
	}, [isPrimary, generateAndAnnounce]);

	useEffect(() => {
		let unlisten: (() => void) | undefined;
		(async () => {
			const off = await listen<TaskReadyPayload>(
				TASK_READY_EVENT,
				(event: Event<TaskReadyPayload>) => {
					setMode({
						kind: "task",
						rerollIndex: event.payload.rerollIndex,
						bundle: event.payload.bundle,
						payload: event.payload.payload,
						disabledUntil: event.payload.disabledUntil,
					});
				},
			);
			unlisten = off;
		})();
		return () => unlisten?.();
	}, []);

	useEffect(() => {
		let unlisten: (() => void) | undefined;
		(async () => {
			const off = await listen(TASK_RESET_EVENT, () => {
				setMode({ kind: "companion" });
			});
			unlisten = off;
		})();
		return () => unlisten?.();
	}, []);

	const finishInterruption = useCallback(
		async (
			outcome: TaskOutcome,
			bundle: GeneratedTaskBundle,
			rerollIndex: number,
		) => {
			if (!isPrimary) return;
			const event = {
				id: newId("evt"),
				created_at: nowIso(),
				source: "ai" as const,
				category: bundle.task.category,
				difficulty: bundle.task.difficulty,
				app_category: null,
				reroll_index: rerollIndex,
				completed: outcome === "completed",
				dismissed: outcome === "dismissed",
				marked_inaccessible: outcome === "inaccessible",
			};
			await recordTaskEvent(event);

			// Hand off to Rust for the actual cat-state evolution. This
			// applies category-specific need decrements, derives mood from
			// state, increments the streak counter, and unlocks tier skills
			// when thresholds cross. We update local state from the response.
			const { cat: updatedCat, effect } = await applyTaskOutcome(
				{
					category: bundle.task.category,
					outcome,
					completed_at: event.created_at,
					completion_line:
						outcome === "completed" ? bundle.completion_line : null,
				},
				event,
			);
			setCat(updatedCat);

			// Always regen on a real outcome (completion or dismissal),
			// regardless of whether the portrait signature changed. gpt-image-2
			// has built-in non-determinism so each call produces a fresh frame
			// — that's the demo behavior we want. Inaccessible feedback skips
			// regen since we don't want to "reward" a "not for me" with a
			// shiny new portrait.
			if (outcome !== "inaccessible") {
				void effect; // signature change tracking now informational only
				regenCatPortrait().catch(() => {
					// Best effort; if it fails we just keep the old portrait.
				});
			}

			// Immediate dismiss on every outcome — the interruption screen
			// disappears as soon as the user acts. Cat reaction lands later
			// in the bottom-right companion when the regen image is ready.
			void effect;
			if (outcome === "completed") {
				completedCategoriesRef.current = [
					bundle.task.category,
					...completedCategoriesRef.current,
				].slice(0, 5);
			} else {
				dismissedCategoriesRef.current = [
					bundle.task.category,
					...dismissedCategoriesRef.current,
				].slice(0, 5);
			}
			await emit(TASK_RESET_EVENT);
			await exitInterruption();
		},
		[isPrimary],
	);

	const reroll = useCallback(
		async (rerollIndex: number, payload: InterruptionPayload) => {
			if (!isPrimary) return;
			setMode({ kind: "loading", rerollIndex: rerollIndex + 1, payload });
			await generateAndAnnounce(rerollIndex + 1, payload);
		},
		[isPrimary, generateAndAnnounce],
	);

	void tick; // re-render every 250ms for the lockout countdown

	return match(mode)
		.with({ kind: "companion" }, () => (
			<CompanionView portraitDataUrl={portraitDataUrl} cat={cat} />
		))
		.with({ kind: "loading" }, () => (
			<InterruptionShell>
				<TaskCardLoading
					portraitDataUrl={portraitDataUrl}
					catName={cat?.name ?? "Your cat"}
				/>
			</InterruptionShell>
		))
		.with({ kind: "task" }, (m) => (
			<InterruptionShell>
				<TaskCard
					bundle={m.bundle}
					rerollIndex={m.rerollIndex}
					disabledUntil={m.disabledUntil}
					portraitDataUrl={portraitDataUrl}
					catName={cat?.name ?? "Your cat"}
					onComplete={() =>
						finishInterruption("completed", m.bundle, m.rerollIndex)
					}
					onReroll={() => reroll(m.rerollIndex, m.payload)}
					onDismiss={() =>
						finishInterruption("dismissed", m.bundle, m.rerollIndex)
					}
					onInaccessible={() =>
						finishInterruption("inaccessible", m.bundle, m.rerollIndex)
					}
				/>
			</InterruptionShell>
		))
		.with({ kind: "error" }, (m) => (
			<InterruptionShell>
				<div className="task-card">
					<p className="task-card-cat-line">The cat is having trouble.</p>
					<p className="task-card-error">{m.message}</p>
					<div className="task-card-actions">
						<button
							type="button"
							className="ghost"
							onClick={async () => {
								await emit(TASK_RESET_EVENT);
								await exitInterruption();
							}}
						>
							Close
						</button>
					</div>
				</div>
			</InterruptionShell>
		))
		.exhaustive();
}

function CompanionView({
	portraitDataUrl,
	cat,
}: {
	portraitDataUrl: string | null;
	cat: Cat | null;
}) {
	// `data-tauri-drag-region` doesn't fire on focusable(false) NSPanels —
	// the panel never receives the mouse events Tauri listens for. We
	// trigger startDragging() ourselves on left-button mousedown.
	const startDrag = (event: React.MouseEvent) => {
		if (event.button !== 0) return;
		void getCurrentWindow().startDragging();
	};
	return (
		<button
			type="button"
			className="companion"
			title={cat?.name ?? "cat"}
			onMouseDown={startDrag}
		>
			<div className="companion-frame">
				{portraitDataUrl ? (
					<img
						src={portraitDataUrl}
						alt={cat?.name ?? "cat"}
						draggable={false}
					/>
				) : (
					<div className="companion-placeholder" />
				)}
			</div>
		</button>
	);
}

function InterruptionShell({ children }: { children: React.ReactNode }) {
	return (
		<div className="interruption-root">
			<div className="interruption-backdrop" />
			<div className="interruption-content">{children}</div>
		</div>
	);
}

function TaskCardLoading({
	portraitDataUrl,
	catName,
}: {
	portraitDataUrl: string | null;
	catName: string;
}) {
	return (
		<div className="task-card task-card-loading">
			<div className="task-card-portrait">
				{portraitDataUrl ? (
					<img src={portraitDataUrl} alt={catName} />
				) : (
					<div className="cat-portrait-placeholder" />
				)}
			</div>
			<p className="task-card-cat-line">
				{catName} is deciding what they want…
			</p>
		</div>
	);
}

function TaskCard({
	bundle,
	rerollIndex,
	disabledUntil,
	portraitDataUrl,
	catName,
	onComplete,
	onReroll,
	onDismiss,
	onInaccessible,
}: {
	bundle: GeneratedTaskBundle;
	rerollIndex: number;
	disabledUntil: number;
	portraitDataUrl: string | null;
	catName: string;
	onComplete: () => void;
	onReroll: () => void;
	onDismiss: () => void;
	onInaccessible: () => void;
}) {
	const remainingMs = Math.max(0, disabledUntil - Date.now());
	const locked = remainingMs > 0;
	return (
		<div className="task-card">
			<div className="task-card-header">
				<div className="task-card-portrait">
					{portraitDataUrl ? (
						<img src={portraitDataUrl} alt={catName} />
					) : (
						<div className="cat-portrait-placeholder" />
					)}
				</div>
				<div>
					<div className="task-card-need">
						{catName} is feeling {bundle.need.replace(/_/g, " ")}.
					</div>
					<p className="task-card-cat-line">{bundle.cat_line}</p>
				</div>
			</div>

			<h2 className="task-card-title">{bundle.task.title}</h2>
			<p className="task-card-instruction">{bundle.task.instruction}</p>

			<div className="task-card-actions">
				<button
					type="button"
					className={clsx("primary task-card-primary", locked && "locked")}
					onClick={onComplete}
					disabled={locked}
				>
					{locked ? `Hold on… ${Math.ceil(remainingMs / 1000)}s` : "I did it."}
				</button>
				<div className="task-card-actions-secondary">
					<button
						type="button"
						className="ghost"
						onClick={onReroll}
						disabled={locked}
					>
						Reroll{rerollIndex >= 4 ? " (easy mode)" : ""}
					</button>
					<button
						type="button"
						className="ghost"
						onClick={onDismiss}
						disabled={locked}
					>
						Not right now
					</button>
				</div>
				<button
					type="button"
					className="task-card-inaccessible"
					onClick={onInaccessible}
					disabled={locked}
				>
					This doesn't work for me
				</button>
			</div>
			{rerollIndex >= 5 ? (
				<p className="task-card-hint">
					The cat picked something tiny. No items, no big movements.
				</p>
			) : null}
		</div>
	);
}

export default OverlayApp;
