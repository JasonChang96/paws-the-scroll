import { preload, removeBackground } from "@imgly/background-removal";

/// Pre-warm the ONNX model and WASM runtime. First call to `removeBackground`
/// would otherwise download ~30 MB of model assets and pay the WASM init
/// cost; calling this at app boot moves that cost off the critical path.
let preloadPromise: Promise<void> | null = null;
export function preloadBackgroundRemoval(): Promise<void> {
	if (!preloadPromise) {
		preloadPromise = preload().catch((error) => {
			console.warn("[bgRemoval] preload failed", error);
			preloadPromise = null;
			throw error;
		});
	}
	return preloadPromise;
}

/// Per-source-data-URL cache. removeBackground takes ~1-3s in the webview;
/// the cached cat portrait gets read every time the overlay re-mounts, so
/// memoizing on the raw data URL keeps the companion responsive after the
/// first strip.
const cache = new Map<string, string>();

/// Pipe a data URL through `@imgly/background-removal`. Returns a new data
/// URL pointing at a PNG with the background masked out. On any failure
/// (model download, runtime error) returns the input unchanged so the user
/// still sees a cat — the demo never blocks on background removal.
export async function stripBackground(sourceDataUrl: string): Promise<string> {
	const cached = cache.get(sourceDataUrl);
	if (cached !== undefined) {
		return cached;
	}
	try {
		const sourceBlob = await fetch(sourceDataUrl).then((r) => r.blob());
		const strippedBlob = await removeBackground(sourceBlob);
		const stripped = await blobToDataUrl(strippedBlob);
		cache.set(sourceDataUrl, stripped);
		return stripped;
	} catch (error) {
		console.warn("[bgRemoval] failed, using opaque source", error);
		return sourceDataUrl;
	}
}

function blobToDataUrl(blob: Blob): Promise<string> {
	return new Promise((resolve, reject) => {
		const reader = new FileReader();
		reader.onload = () => resolve(reader.result as string);
		reader.onerror = () => reject(reader.error);
		reader.readAsDataURL(blob);
	});
}
