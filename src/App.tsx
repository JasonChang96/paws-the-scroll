import { useEffect } from "react";
import { match } from "ts-pattern";
import "./App.css";
import { getCat, getSettings } from "./lib/api";
import { preloadBackgroundRemoval } from "./lib/backgroundRemoval";
import { useViewStore } from "./lib/viewStore";
import { CatAdoption } from "./views/CatAdoption";
import { Dashboard } from "./views/Dashboard";
import { Onboarding } from "./views/Onboarding";
import { Settings } from "./views/Settings";

function App() {
	const view = useViewStore((s) => s.view);
	const setView = useViewStore((s) => s.setView);

	useEffect(() => {
		// Kick off the ~30 MB model download in the background while the
		// user is reading the welcome / picking a cat. Errors are swallowed
		// inside `preloadBackgroundRemoval`; the strip path falls back to
		// the opaque source if the model never finishes loading.
		void preloadBackgroundRemoval();
		(async () => {
			try {
				const settings = await getSettings();
				if (!settings.onboarding_complete) {
					setView("onboarding");
					return;
				}
				const cat = await getCat();
				setView(cat ? "dashboard" : "adopt");
			} catch {
				setView("onboarding");
			}
		})();
	}, [setView]);

	return match(view)
		.with("loading", () => (
			<div className="loading-screen">
				<p>Looking for your cat…</p>
			</div>
		))
		.with("onboarding", () => <Onboarding />)
		.with("adopt", () => <CatAdoption />)
		.with("dashboard", () => <Dashboard />)
		.with("settings", () => <Settings />)
		.exhaustive();
}

export default App;
