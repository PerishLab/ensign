export type Client =
	| { status: "ok"; body: Record<string, unknown> }
	| { status: "absent" }
	| { status: "unknown" };

let cache = new WeakMap<typeof fetch, Promise<Client>>();

function record(value: unknown): value is Record<string, unknown> {
	return typeof value === "object" && value !== null && !Array.isArray(value);
}

async function read(fetcher: typeof fetch): Promise<Client> {
	try {
		const response = await fetcher("/client.json", {
			cache: "no-store",
			credentials: "omit",
			redirect: "error",
			signal: AbortSignal.timeout(15000),
		});
		if (response.status === 404) return { status: "absent" };
		if (!response.ok) return { status: "unknown" };
		const body: unknown = await response.json();
		if (!record(body)) return { status: "unknown" };
		return { status: "ok", body };
	} catch {
		return { status: "unknown" };
	}
}

export function load(fetcher: typeof fetch = fetch): Promise<Client> {
	const held = cache.get(fetcher);
	if (held) return held;
	const pending = read(fetcher).then((result) => {
		if (result.status === "unknown") cache.delete(fetcher);
		return result;
	});
	cache.set(fetcher, pending);
	return pending;
}

export function clear(fetcher?: typeof fetch): void {
	if (fetcher) {
		cache.delete(fetcher);
		return;
	}
	cache = new WeakMap();
}
