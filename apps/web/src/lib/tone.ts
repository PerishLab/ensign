export type Tone = "light" | "dark";

const slot = "ensign.tone";

export function stored(): Tone | undefined {
	try {
		const raw = globalThis.localStorage?.getItem(slot);
		if (raw === "light" || raw === "dark") return raw;
	} catch {}
	return undefined;
}

export function read(): Tone {
	const kept = stored();
	if (kept !== undefined) return kept;
	return globalThis.matchMedia?.("(prefers-color-scheme: dark)").matches
		? "dark"
		: "light";
}

export function save(next: Tone): void {
	try {
		globalThis.localStorage?.setItem(slot, next);
	} catch {}
}

export function watch(change: (next: Tone) => void): () => void {
	const media = globalThis.matchMedia?.("(prefers-color-scheme: dark)");
	if (!media) return () => {};
	const handle = () => {
		if (stored() === undefined) change(media.matches ? "dark" : "light");
	};
	media.addEventListener("change", handle);
	return () => media.removeEventListener("change", handle);
}
