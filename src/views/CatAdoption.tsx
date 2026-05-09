import clsx from "clsx";
import { useState } from "react";
import beanPng from "../../assets/bean.png";
import mangoPng from "../../assets/mango.png";
import plutoPng from "../../assets/pluto.png";
import { ErrorModal } from "../components/ErrorModal";
import { readPortraitBytes, saveCat, seedInitialPortrait } from "../lib/api";
import type { Cat, CatType } from "../lib/types";
import { newId } from "../lib/util";
import { useViewStore } from "../lib/viewStore";

interface CatChoice {
	type: CatType;
	displayName: string;
	tagline: string;
	hint: string;
	baseImage: string;
	visibleTraits: string[];
	hiddenTraits: string[];
}

// Hand-drawn base portraits shown on the picker cards. The same image is the
// "before" frame for every `gpt-image-2` edit call once the cat starts
// evolving, which keeps the visual style consistent without us re-pinning a
// style guide in every prompt.
const CHOICES: CatChoice[] = [
	{
		type: "orange_fat",
		displayName: "Mango",
		tagline: "An orange situation.",
		hint: "Food-motivated. Very dramatic. Will tell you exactly what they want.",
		baseImage: mangoPng,
		visibleTraits: ["theatrical", "snack-oriented", "vocal"],
		hiddenTraits: ["needs reassurance", "secretly tender"],
	},
	{
		type: "void",
		displayName: "Pluto",
		tagline: "Something quiet sat near you.",
		hint: "Quiet. Watching. Affectionate, eventually.",
		baseImage: plutoPng,
		visibleTraits: ["observant", "still", "mysterious"],
		hiddenTraits: ["very loyal", "playful at night"],
	},
	{
		type: "scrungly_street",
		displayName: "Bean",
		tagline: "Slightly scuffed. Definitely yours.",
		hint: "Scrappy. Loud. Suspiciously loyal.",
		baseImage: beanPng,
		visibleTraits: ["chaotic", "scrappy", "alert"],
		hiddenTraits: ["resilient", "weirdly tender about routine"],
	},
];

export function CatAdoption() {
	const setView = useViewStore((s) => s.setView);
	const [pending, setPending] = useState<CatType | null>(null);
	const [error, setError] = useState<string | null>(null);
	const [revealed, setRevealed] = useState<{
		choice: CatChoice;
		dataUrl: string;
		cat: Cat;
	} | null>(null);

	const adopt = async (choice: CatChoice) => {
		setPending(choice.type);
		setError(null);
		try {
			const catId = newId("cat");
			// Seed the initial portrait from the embedded base PNG — no
			// OpenAI call. The base image already represents the cat's
			// starting state; first edit-API regeneration only happens
			// later when mood/tier/skills change.
			const portrait = await seedInitialPortrait(catId, choice.type);
			const b64 = await readPortraitBytes(portrait.path);
			const cat: Cat = {
				id: catId,
				type: choice.type,
				name: choice.displayName,
				visible_traits: choice.visibleTraits,
				hidden_traits: choice.hiddenTraits,
				needs: {
					hunger: 0.3,
					boredom: 0.3,
					loneliness: 0.2,
					dirty_litter: 0.1,
					play_drive: 0.4,
					attention: 0.5,
				},
				mood: "content",
				independence_level: 0,
				skills: [],
				items: [],
				story_events: [],
				portrait_path: portrait.path,
			};
			await saveCat(cat);
			setRevealed({ choice, dataUrl: `data:image/jpeg;base64,${b64}`, cat });
		} catch (e) {
			setError(e instanceof Error ? e.message : String(e));
		} finally {
			setPending(null);
		}
	};

	if (revealed) {
		return (
			<div className="adoption">
				<div className="adoption-reveal">
					<div className="reveal-frame">
						<img src={revealed.dataUrl} alt={revealed.choice.displayName} />
					</div>
					<h1>{revealed.choice.displayName} chose you.</h1>
					<p className="lede">{revealed.choice.tagline}</p>
					<p className="muted">{revealed.choice.hint}</p>
					<button
						type="button"
						className="primary"
						onClick={() => setView("dashboard")}
					>
						Take me home
					</button>
				</div>
			</div>
		);
	}

	return (
		<div className="adoption">
			<header className="adoption-header">
				<h1>Three cats came by.</h1>
				<p className="muted">One of them will choose you.</p>
			</header>
			<div className="cat-grid">
				{CHOICES.map((choice) => (
					<button
						type="button"
						key={choice.type}
						className={clsx("cat-card", pending === choice.type && "loading")}
						onClick={() => adopt(choice)}
						disabled={pending !== null}
					>
						<div className="cat-card-art">
							<img
								src={choice.baseImage}
								alt={choice.displayName}
								className="cat-card-base"
							/>
						</div>
						<div className="cat-card-meta">
							<div className="cat-card-name">{choice.displayName}</div>
							<div className="cat-card-tagline">{choice.tagline}</div>
							<div className="cat-card-hint">{choice.hint}</div>
						</div>
						{pending === choice.type ? (
							<div className="cat-card-pending">Drawing your cat…</div>
						) : null}
					</button>
				))}
			</div>
			<ErrorModal
				message={error}
				onDismiss={() => setError(null)}
				title="The cat couldn't quite arrive."
			/>
		</div>
	);
}
