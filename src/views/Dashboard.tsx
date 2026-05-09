import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";
import beanPng from "../../assets/bean.png";
import mangoPng from "../../assets/mango.png";
import plutoPng from "../../assets/pluto.png";
import {
	getCat,
	listAggregates,
	listTaskEvents,
	readPortraitBytes,
} from "../lib/api";
import { stripBackground } from "../lib/backgroundRemoval";
import type { ActivityAggregate, Cat, TaskEvent } from "../lib/types";
import { useViewStore } from "../lib/viewStore";

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

export function Dashboard() {
	const setView = useViewStore((s) => s.setView);
	const [cat, setCat] = useState<Cat | null>(null);
	const [portraitDataUrl, setPortraitDataUrl] = useState<string | null>(null);
	const [aggregates, setAggregates] = useState<ActivityAggregate[]>([]);
	const [events, setEvents] = useState<TaskEvent[]>([]);

	const currentPortraitRef = useRef<string | null>(null);
	useEffect(() => {
		currentPortraitRef.current = portraitDataUrl;
	}, [portraitDataUrl]);

	const refreshAll = useCallback(async () => {
		const [c, a, e] = await Promise.all([
			getCat(),
			listAggregates(),
			listTaskEvents(),
		]);
		setCat(c);
		setAggregates(a);
		setEvents(e);
		if (!c?.portrait_path) {
			setPortraitDataUrl(null);
			return;
		}
		try {
			const raw = await readPortraitBytes(c.portrait_path);
			// Skip bg removal when still on the base PNG (already
			// transparent). Same logic as the overlay.
			if (c.portrait_is_base) {
				setPortraitDataUrl(raw);
				return;
			}
			if (currentPortraitRef.current === null) {
				setPortraitDataUrl(baseImageFor(c.type));
			}
			setPortraitDataUrl(await stripBackground(raw));
		} catch {
			setPortraitDataUrl(null);
		}
	}, []);

	useEffect(() => {
		void refreshAll();
	}, [refreshAll]);

	// Re-fetch whenever Rust persists a new cat (task outcome regen,
	// time-away rewards, evolve shortcut). Same plumbing as OverlayApp.
	useEffect(() => {
		let unlisten: (() => void) | undefined;
		(async () => {
			unlisten = await listen("cat-updated", () => {
				void refreshAll();
			});
		})();
		return () => unlisten?.();
	}, [refreshAll]);

	if (!cat) {
		return (
			<div className="dashboard">
				<p>Looking for your cat…</p>
			</div>
		);
	}

	const today = aggregates.find(
		(a) => a.date === new Date().toISOString().slice(0, 10),
	);
	const totalCompleted = events.filter((e) => e.completed).length;
	const totalRerolls = events.reduce((acc, e) => acc + e.reroll_index, 0);
	const totalInterruptions = aggregates.reduce(
		(acc, a) => acc + a.interruptions,
		0,
	);
	const totalAwayMinutes = Math.floor(
		aggregates.reduce(
			(acc, a) => acc + a.time_away_after_interruptions_seconds,
			0,
		) / 60,
	);
	const todayActiveMinutes = Math.floor((today?.active_seconds ?? 0) / 60);
	const todaySocialMinutes = Math.floor((today?.social_seconds ?? 0) / 60);

	return (
		<div className="dashboard">
			<header className="dashboard-header">
				<div className="cat-portrait">
					{portraitDataUrl ? (
						<img src={portraitDataUrl} alt={cat.name} />
					) : (
						<div className="cat-portrait-placeholder" />
					)}
				</div>
				<div>
					<h1>{cat.name}</h1>
					<p className="muted">
						Mood: <strong>{cat.mood}</strong> · Independence{" "}
						{Math.round(cat.independence_level * 100)}%
					</p>
				</div>
				<button
					type="button"
					className="ghost"
					onClick={() => setView("settings")}
				>
					Settings
				</button>
			</header>

			<section className="needs-grid">
				<NeedBar label="Hungry" value={cat.needs.hunger} />
				<NeedBar label="Bored" value={cat.needs.boredom} />
				<NeedBar label="Lonely" value={cat.needs.loneliness} />
				<NeedBar label="Litter" value={cat.needs.dirty_litter} />
				<NeedBar label="Play" value={cat.needs.play_drive} />
				<NeedBar label="Attention" value={cat.needs.attention} />
			</section>

			<section className="metrics-grid">
				<Metric label="Spirals interrupted" value={totalInterruptions} />
				<Metric label="Tiny actions completed" value={totalCompleted} />
				<Metric
					label="Time your cat spent thriving without you"
					value={`${totalAwayMinutes}m`}
				/>
				<Metric label="Rerolls (the cat is patient)" value={totalRerolls} />
				<Metric label="Today's active time" value={`${todayActiveMinutes}m`} />
				<Metric
					label="Today's social-app time"
					value={`${todaySocialMinutes}m`}
				/>
			</section>

			<section>
				<h2>Recent story moments</h2>
				{cat.story_events.length === 0 ? (
					<p className="muted">
						No story yet. The cat will start collecting moments as you spend
						time apart.
					</p>
				) : (
					<ul className="story-list">
						{cat.story_events
							.slice(-5)
							.reverse()
							.map((s) => (
								<li key={s.id}>
									<time>{new Date(s.at).toLocaleString()}</time>
									<span>{s.text}</span>
								</li>
							))}
					</ul>
				)}
			</section>
		</div>
	);
}

function NeedBar({ label, value }: { label: string; value: number }) {
	const pct = Math.max(0, Math.min(1, value)) * 100;
	return (
		<div className="need-bar">
			<div className="need-bar-label">{label}</div>
			<div className="need-bar-track">
				<div className="need-bar-fill" style={{ width: `${pct}%` }} />
			</div>
		</div>
	);
}

function Metric({ label, value }: { label: string; value: string | number }) {
	return (
		<div className="metric">
			<div className="metric-value">{value}</div>
			<div className="metric-label">{label}</div>
		</div>
	);
}
