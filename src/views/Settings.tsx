import { useEffect, useState } from "react";
import { ErrorModal } from "../components/ErrorModal";
import { getSettings, saveSettings } from "../lib/api";
import type { Settings as SettingsModel } from "../lib/types";
import { useViewStore } from "../lib/viewStore";
import { CrisisNote } from "./Onboarding";

export function Settings() {
	const setView = useViewStore((s) => s.setView);
	const [settings, setSettings] = useState<SettingsModel | null>(null);
	const [savedAt, setSavedAt] = useState<number | null>(null);
	const [error, setError] = useState<string | null>(null);

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
					Shorten the grace period and interruption window for stage demos.
				</p>
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
