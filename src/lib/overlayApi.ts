import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
	AppCategory,
	ForegroundApp,
	InterruptionSource,
	OverlayMode,
	TimeOfDay,
} from "./types";

export interface InterruptionPayload {
	source: InterruptionSource;
	active_app: ForegroundApp | null;
	active_app_category: AppCategory;
	time_of_day_label: TimeOfDay;
	active_streak_seconds: number;
	today_active_seconds: number;
	today_social_seconds: number;
	today_interruptions: number;
	today_completed: number;
	today_dismissed: number;
}

export type { OverlayMode };

export const onInterruptionRequested = (
	cb: (payload: InterruptionPayload) => void,
): Promise<UnlistenFn> =>
	listen<InterruptionPayload>("interruption-requested", (event) =>
		cb(event.payload),
	);

export const onOverlayModeChanged = (
	cb: (mode: OverlayMode) => void,
): Promise<UnlistenFn> =>
	listen<OverlayMode>("overlay-mode-changed", (event) => cb(event.payload));

export const enterInterruption = (): Promise<void> =>
	invoke("enter_interruption_mode");

export const exitInterruption = (): Promise<void> =>
	invoke("exit_interruption_mode");

export const requestInterruption = (): Promise<void> =>
	invoke("request_interruption");
