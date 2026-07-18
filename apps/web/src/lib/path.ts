export function back(): string {
	const held = seek();
	return held ?? "/portal";
}

export function carry(): string {
	const held = seek();
	return held ? `?return=${encodeURIComponent(held)}` : "";
}

function seek(): string | null {
	const search = globalThis.location?.search ?? "";
	const held = new URLSearchParams(search).get("return");
	if (!held?.startsWith("/")) {
		return null;
	}
	const home = globalThis.location?.origin ?? "http://127.0.0.1";
	let sail: URL;
	try {
		sail = new URL(held, home);
	} catch {
		return null;
	}
	if (sail.origin !== home) {
		return null;
	}
	const path = sail.pathname + sail.search;
	if (!path.startsWith("/") || path.startsWith("//")) {
		return null;
	}
	return path;
}
