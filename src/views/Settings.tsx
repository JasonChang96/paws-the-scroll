import { useEffect, useState } from "react";
import { ErrorModal } from "../components/ErrorModal";
import {
	demoSetCatMood,
	factoryReset,
	getSettings,
	saveSettings,
} from "../lib/api";
import type { CatMood, Settings as SettingsModel } from "../lib/types";
import { useViewStore } from "../lib/viewStore";
import { CrisisNote } from "./Onboarding";

const DEMO_MOODS: Array<{ mood: CatMood; label: string }> = [
	{ mood: "content", label: "Content" },
	{ mood: "peckish", label: "Peckish" },
	{ mood: "hungry", label: "Hungry" },
	{ mood: "lonely", label: "Lonely" },
	{ mood: "restless", label: "Restless" },
	{ mood: "playful", label: "Playful" },
	{ mood: "unkempt", label: "Unkempt" },
	{ mood: "demanding", label: "Demanding" },
	{ mood: "smug", label: "Smug" },
	{ mood: "sulky", label: "Sulky" },
	{ mood: "excited", label: "Excited" },
	{ mood: "dramatic", label: "Dramatic" },
	{ mood: "sleepy", label: "Sleepy" },
	{ mood: "affectionate", label: "Affectionate" },
];

export function Settings() {
	const setView = useViewStore((s) => s.setView);
	const [settings, setSettings] = useState<SettingsModel | null>(null);
	const [savedAt, setSavedAt] = useState<number | null>(null);
	const [error, setError] = useState<string | null>(null);
	const [confirmingReset, setConfirmingReset] = useState(false);
	const [resetting, setResetting] = useState(false);
	const [settingMood, setSettingMood] = useState<CatMood | null>(null);

	useEffect(() => {
		getSettings()
			.then(setSettings)
			.catch((e) => setError(e instanceof Error ? e.message : String(e)));
	}, []);

	const persist = async (next: SettingsModel) => {
		setSettings(next);
		try {
			await saveSettings(next);
			setSavedAt(Date.now());
		} catch (e) {
			setError(e instanceof Error ? e.message : String(e));
		}
	};

	if (!settings) {
		return (
			<div className="settings">
				<p>{error ?? "Loading…"}</p>
			</div>
		);
	}

	return (
		<div className="settings">
			<header>
				<h1>Settings</h1>
				<button
					type="button"
					className="ghost"
					onClick={() => setView("dashboard")}
				>
					Back to cat
				</button>
			</header>

			<section>
				<h2>OpenAI key</h2>
				<p className="muted">
					Stored locally only. The cat needs this to talk and to draw itself.
				</p>
				<input
					type="password"
					placeholder="sk-..."
					className="text-input"
					value={settings.openai_api_key ?? ""}
					onChange={(e) =>
						persist({
							...settings,
							openai_api_key: e.currentTarget.value || null,
						})
					}
				/>
			</section>

			<section>
				<h2>Demo mode</h2>
				<p className="muted">
					Disables automatic interruptions. The activity scheduler keeps
					tracking time, but the cat only fires when you trigger it yourself —
					keyboard shortcuts make demos deterministic and on-cue.
				</p>
				<ul className="muted shortcut-list">
					<li>
						<kbd>⌘</kbd> <kbd>⌃</kbd> <kbd>⌥</kbd> <kbd>P</kbd>
						<span>Summon the cat (fire interruption now)</span>
					</li>
					<li>
						<kbd>⌘</kbd> <kbd>⌃</kbd> <kbd>⌥</kbd> <kbd>E</kbd>
						<span>Evolve the cat one tier (regen portrait)</span>
					</li>
				</ul>
				<label className="toggle-row">
					<input
						type="checkbox"
						checked={settings.demo_mode}
						onChange={(e) =>
							persist({ ...settings, demo_mode: e.currentTarget.checked })
						}
					/>
					<span>Demo mode</span>
				</label>
				<div className="demo-mood-controls">
					{DEMO_MOODS.map(({ mood, label }) => (
						<button
							type="button"
							className="ghost"
							key={mood}
							onClick={async () => {
								setSettingMood(mood);
								try {
									await demoSetCatMood(mood);
								} catch (e) {
									setError(e instanceof Error ? e.message : String(e));
								} finally {
									setSettingMood(null);
								}
							}}
							disabled={settingMood !== null}
						>
							{settingMood === mood ? "Setting…" : label}
						</button>
					))}
				</div>
			</section>

			<section>
				<h2>Timing</h2>
				<NumberRow
					label="Grace period (seconds)"
					value={settings.grace_period_seconds}
					onChange={(v) => persist({ ...settings, grace_period_seconds: v })}
				/>
				<NumberRow
					label="Idle threshold (seconds)"
					value={settings.idle_threshold_seconds}
					onChange={(v) => persist({ ...settings, idle_threshold_seconds: v })}
				/>
				<NumberRow
					label="Interruption window min (seconds)"
					value={settings.interruption_window_min_seconds}
					onChange={(v) =>
						persist({ ...settings, interruption_window_min_seconds: v })
					}
				/>
				<NumberRow
					label="Interruption window max (seconds)"
					value={settings.interruption_window_max_seconds}
					onChange={(v) =>
						persist({ ...settings, interruption_window_max_seconds: v })
					}
				/>
			</section>

			<CrisisNote />

			<section className="danger-zone">
				<h2>Reset</h2>
				<p className="muted">
					Wipes your profile, your cat, all task history, and the cached cat
					portraits. The cat won't remember you. Useful while we're still
					iterating on the app.
				</p>
				<button
					type="button"
					className="ghost danger-button"
					onClick={() => setConfirmingReset(true)}
					disabled={resetting}
				>
					{resetting ? "Resetting…" : "Factory reset"}
				</button>
			</section>

			{confirmingReset ? (
				<button
					type="button"
					className="error-modal-backdrop"
					onClick={() => setConfirmingReset(false)}
					aria-label="Cancel reset"
				>
					<div
						className="error-modal-card"
						role="alertdialog"
						aria-modal="true"
						aria-labelledby="reset-confirm-title"
						onClick={(event) => event.stopPropagation()}
						onKeyDown={(event) => event.stopPropagation()}
					>
						<div className="error-modal-icon" aria-hidden="true">
							🐾
						</div>
						<h2 id="reset-confirm-title" className="error-modal-title">
							Erase everything?
						</h2>
						<p className="error-modal-message">
							Your cat will be gone. The portraits, the streak, the story — all
							of it. You'll start over from onboarding.
						</p>
						<div className="confirm-button-row">
							<button
								type="button"
								className="ghost"
								onClick={() => setConfirmingReset(false)}
								disabled={resetting}
							>
								Keep my cat
							</button>
							<button
								type="button"
								className="primary danger-button"
								onClick={async () => {
									setResetting(true);
									try {
										await factoryReset();
										setView("onboarding");
									} catch (e) {
										setError(e instanceof Error ? e.message : String(e));
										setConfirmingReset(false);
									} finally {
										setResetting(false);
									}
								}}
								disabled={resetting}
							>
								{resetting ? "Erasing…" : "Erase"}
							</button>
						</div>
					</div>
				</button>
			) : null}

			{savedAt ? <p className="muted small">Saved.</p> : null}
			<ErrorModal
				message={error}
				onDismiss={() => setError(null)}
				title="Couldn't save settings."
			/>
		</div>
	);
}

function NumberRow({
	label,
	value,
	onChange,
}: {
	label: string;
	value: number;
	onChange: (v: number) => void;
}) {
	return (
		<label className="number-row">
			<span>{label}</span>
			<input
				type="number"
				min={0}
				value={value}
				onChange={(e) => onChange(Number(e.currentTarget.value))}
			/>
		</label>
	);
}
