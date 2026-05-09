export function newId(prefix: string): string {
	return `${prefix}_${crypto.randomUUID()}`;
}

export function nowIso(): string {
	return new Date().toISOString();
}

export function independenceTier(level: number): number {
	if (level >= 0.75) return 3;
	if (level >= 0.5) return 2;
	if (level >= 0.25) return 1;
	return 0;
}
