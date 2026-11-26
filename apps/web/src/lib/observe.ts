import { capture, type Fetcher, Locus } from "@perishlab/locus";

const producer = "ensign-web";

let client = new Locus({ producer, enabled: false });
let detach: (() => void) | undefined;

export function locus(): Locus {
	return client;
}

function clean(text: string): boolean {
	if (!text || /\s/u.test(text) || text.includes("\\")) return false;
	for (const char of text) {
		const point = char.codePointAt(0) ?? 0;
		if (point < 32 || (point >= 127 && point <= 159)) return false;
	}
	return true;
}

export function endpoint(raw: unknown): string | undefined {
	if (typeof raw !== "string") return undefined;
	const text = raw.trim();
	if (!text.startsWith("/") || text.startsWith("//")) return undefined;
	if (!clean(text)) return undefined;
	return text;
}

export function activate(target?: string, fetcher?: Fetcher): boolean {
	detach?.();
	detach = undefined;
	void client.close();
	const valid = endpoint(target);
	if (!valid) {
		client = new Locus({ producer, enabled: false });
		return false;
	}
	client = new Locus(
		fetcher
			? { producer, endpoint: valid, fetcher }
			: { producer, endpoint: valid },
	);
	detach = capture(client);
	return true;
}

export async function flush(): Promise<void> {
	await client.flush();
}

export async function reset(): Promise<void> {
	detach?.();
	detach = undefined;
	await client.close();
	client = new Locus({ producer, enabled: false });
}

export async function boot(fetcher: typeof fetch = fetch): Promise<boolean> {
	try {
		const response = await fetcher("/client.json", {
			cache: "no-store",
			credentials: "omit",
			redirect: "error",
			signal: AbortSignal.timeout(15000),
		});
		if (!response.ok) return false;
		const body: unknown = await response.json();
		if (typeof body !== "object" || body === null || Array.isArray(body)) {
			return false;
		}
		const target = (body as Record<string, unknown>).observe;
		return typeof target === "string" ? activate(target) : false;
	} catch {
		return false;
	}
}

export function route(raw: string): string {
	const path = raw.split(/[?#]/u, 1)[0] ?? "";
	return path.replace(/\/\d+(?=\/|$)/gu, "/:id");
}

export function info(text: string): void {
	client.info(text);
}

export function warn(text: string): void {
	client.warn(text);
}

export function count(flow: string, status: string, path = ""): void {
	const labels: Record<string, string> = path
		? { flow, status, route: route(path) }
		: { flow, status };
	client.metric({
		name: "ensign.api.error",
		unit: "count",
		kind: "counter",
		series: producer,
		value: 1,
		labels,
	});
}

function record(
	path: string,
	method: string,
	flow: string,
	status: number,
): void {
	if (status >= 500) {
		client.fault(new Error(`HTTP ${status}`), {
			route: path,
			method,
			flow,
			status,
		});
		count(flow, String(status), path);
		return;
	}
	if (status >= 400) {
		client.warn(`${flow} ${path} ${status}`);
		count(flow, String(status), path);
		return;
	}
	if (method !== "GET") {
		client.info(`${flow} ${path} ${status}`);
	}
}

export async function track(
	path: string,
	method: string,
	flow: string,
	work: () => Promise<Response>,
): Promise<Response> {
	const cleanpath = route(path);
	const started = performance.now();
	const run = client.span("api.request", work);
	try {
		const response = await client.with({ route: cleanpath, method, flow }, () =>
			run(),
		);
		const elapsed = Math.max(0, performance.now() - started);
		client.metric({
			name: "ensign.api.latency",
			unit: "ms",
			kind: "gauge",
			series: producer,
			value: elapsed,
			labels: { route: cleanpath, flow, status: String(response.status) },
		});
		record(cleanpath, method, flow, response.status);
		return response;
	} catch (reason) {
		const name = reason instanceof Error ? reason.name : "network";
		client.fault(new Error(name), {
			route: cleanpath,
			method,
			flow,
			status: 0,
		});
		count(flow, "net", cleanpath);
		throw reason;
	}
}
