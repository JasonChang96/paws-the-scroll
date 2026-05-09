import { create } from "zustand";

export type View =
	| "loading"
	| "onboarding"
	| "adopt"
	| "dashboard"
	| "settings";

interface ViewState {
	view: View;
	setView: (view: View) => void;
}

export const useViewStore = create<ViewState>((set) => ({
	view: "loading",
	setView: (view) => set({ view }),
}));
