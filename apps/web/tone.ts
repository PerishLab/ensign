export type Tone = "light" | "dark";

const slot = "ensign.tone";

type Vault = Pick<Storage, "getItem" | "setItem" | "removeItem">;
type Media = Pick<
	MediaQueryList,
	"matches" | "addEventListener" | "removeEventListener"
>;

function media(): Media | undefined {
	return globalThis.matchMedia?.("(prefers-color-scheme: dark)");
}

function vault(storage?: Vault): Vault | undefined {
	if (storage !== undefined) return storage;
	try {
		return globalThis.localStorage;
	} catch {
		return undefined;
	}
}

export function stored(storage?: Vault): Tone | undefined {
	try {
		const raw = vault(storage)?.getItem(slot);
		if (raw === "light" || raw === "dark") return raw;
	} catch {}
	return undefined;
}

export function read(storage?: Vault, source?: Media): Tone {
	const kept = stored(storage);
	if (kept !== undefined) return kept;
	return (source ?? media())?.matches ? "dark" : "light";
}

export function save(next: Tone, storage?: Vault): void {
	try {
		vault(storage)?.setItem(slot, next);
	} catch {}
}

export function watch(
	change: (next: Tone) => void,
	source?: Media,
	storage?: Vault,
): () => void {
	const query = source ?? media();
	if (!query) return () => {};
	const handle = () => {
		if (stored(storage) === undefined) change(query.matches ? "dark" : "light");
	};
	query.addEventListener("change", handle);
	return () => query.removeEventListener("change", handle);
}
