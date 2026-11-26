import { load } from "./src/lib/client.ts";
import { count, info, track, warn } from "./src/lib/observe.ts";

export class Refusal extends Error {}

export type Config = { issuer: string; client: string; folio?: string };
export type Surface = { system?: string; tone?: "light" | "dark" };
export type Lookup =
	| { status: "configured"; href: string }
	| { status: "absent" }
	| { status: "unknown" };
export type Runtime = {
	config?: Config;
	folio: Lookup;
	fault?: Refusal;
};
type Vault = Pick<Storage, "getItem" | "setItem" | "removeItem">;
type Seat = { origin: string; back: string; storage: Vault; crypto: Crypto };
type Journey = Config & {
	origin: string;
	back: string;
	state: string;
	verifier: string;
};

const slot = "ensign.authorization";
let access: { value: string; until: number } | undefined;
let surface: Surface | undefined;

export function held(): string | undefined {
	return access && access.until > Date.now() ? access.value : undefined;
}

export function shown(): Surface | undefined {
	return surface;
}

export function wear(next: Surface | undefined): void {
	surface = next;
}

export function forget(): void {
	access = undefined;
	surface = undefined;
	globalThis.sessionStorage?.removeItem(slot);
	info("session.clear");
}

function record(value: unknown): value is Record<string, unknown> {
	return typeof value === "object" && value !== null && !Array.isArray(value);
}

function clean(text: string): boolean {
	if (!text || /\s/u.test(text)) return false;
	for (const char of text) {
		const point = char.codePointAt(0) ?? 0;
		if (point < 32 || (point >= 127 && point <= 159)) return false;
	}
	return true;
}

export function portal(raw: unknown): string | undefined {
	if (typeof raw !== "string") return undefined;
	const text = raw.trim();
	if (!clean(text)) return undefined;
	try {
		const url = new URL(text);
		if (url.protocol !== "http:" && url.protocol !== "https:") return undefined;
		if (url.username || url.password || !url.hostname) return undefined;
		return url.href;
	} catch {
		return undefined;
	}
}

async function manifest(
	fetcher: typeof fetch,
): Promise<{ body?: Record<string, unknown>; folio: Lookup }> {
	const loaded = await load(fetcher);
	if (loaded.status !== "ok") return { folio: { status: loaded.status } };
	const href = portal(loaded.body.folio);
	return {
		body: loaded.body,
		folio: href ? { status: "configured", href } : { status: "absent" },
	};
}

async function verify(
	issuer: string,
	fetcher: typeof fetch,
): Promise<Refusal | undefined> {
	try {
		const discovery = await fetcher("/api/.well-known/openid-configuration", {
			cache: "no-store",
			credentials: "omit",
			redirect: "error",
			signal: AbortSignal.timeout(15000),
		});
		if (!discovery.ok) return new Refusal("暂时无法确认设置身份，请重试。");
		const known = await discovery.json();
		if (!record(known)) return new Refusal("无法确认设置服务的身份。");
		if (
			known.issuer !== issuer ||
			known.authorization_endpoint !== `${issuer}/authorize` ||
			known.token_endpoint !== `${issuer}/token`
		) {
			return new Refusal("无法确认设置服务的身份。");
		}
		return undefined;
	} catch {
		return new Refusal("暂时无法确认设置身份，请重试。");
	}
}

export async function folio(fetcher: typeof fetch = fetch): Promise<Lookup> {
	return (await manifest(fetcher)).folio;
}

export async function runtime(fetcher: typeof fetch = fetch): Promise<Runtime> {
	const loaded = await manifest(fetcher);
	if (!loaded.body) {
		return {
			folio: loaded.folio,
			fault: new Refusal("暂时无法读取设置，请重试。"),
		};
	}
	const body = loaded.body;
	if (!body.issuer && !body.client) return { folio: loaded.folio };
	if (
		typeof body.issuer !== "string" ||
		typeof body.client !== "string" ||
		!body.client
	) {
		return {
			folio: loaded.folio,
			fault: new Refusal("暂时无法确认设置身份，请重试。"),
		};
	}
	const fault = await verify(body.issuer, fetcher);
	if (fault) return { folio: loaded.folio, fault };
	const href =
		loaded.folio.status === "configured" ? loaded.folio.href : undefined;
	const config: Config = href
		? { issuer: body.issuer, client: body.client, folio: href }
		: { issuer: body.issuer, client: body.client };
	return { config, folio: loaded.folio };
}

export async function configuration(
	fetcher: typeof fetch = fetch,
): Promise<Config | undefined> {
	const loaded = await runtime(fetcher);
	if (loaded.fault) throw loaded.fault;
	return loaded.config;
}

function encode(bytes: Uint8Array): string {
	return btoa(String.fromCharCode(...bytes))
		.replaceAll("+", "-")
		.replaceAll("/", "_")
		.replaceAll("=", "");
}

function valid(path: string): boolean {
	return [
		"/",
		"/login",
		"/settings",
		"/security",
		"/sessions",
		"/admin/actors",
		"/admin/invites",
		"/admin/teams",
		"/admin/apps",
	].includes(path);
}

export async function start(config: Config, seat: Seat): Promise<string> {
	if (!valid(seat.back)) throw new Refusal("返回地址无效。");
	const verifier = encode(seat.crypto.getRandomValues(new Uint8Array(48)));
	const state = encode(seat.crypto.getRandomValues(new Uint8Array(32)));
	const hash = await seat.crypto.subtle.digest(
		"SHA-256",
		new TextEncoder().encode(verifier),
	);
	const journey: Journey = {
		...config,
		origin: seat.origin,
		back: seat.back,
		state,
		verifier,
	};
	seat.storage.setItem(slot, JSON.stringify(journey));
	info("oidc.start");
	const target = new URL("/api/authorize", seat.origin);
	target.search = new URLSearchParams({
		response_type: "code",
		client_id: config.client,
		redirect_uri: `${seat.origin}/callback`,
		scope: "openid profile",
		resource: "folio",
		state,
		code_challenge: encode(new Uint8Array(hash)),
		code_challenge_method: "S256",
	}).toString();
	return target.href;
}

function single(query: URLSearchParams, key: string): string | undefined {
	const values = query.getAll(key);
	if (values.length !== 1) return undefined;
	return values[0] || undefined;
}

export async function complete(
	config: Config,
	seat: { url: string; storage: Vault; fetcher: typeof fetch },
): Promise<string> {
	const raw = seat.storage.getItem(slot);
	seat.storage.removeItem(slot);
	access = undefined;
	if (!raw) {
		warn("oidc.expired");
		count("oidc", "expired", "/callback");
		throw new Refusal("身份确认已失效，请返回登录重试。");
	}
	const journey: Journey = JSON.parse(raw);
	const url = new URL(seat.url);
	const query = url.searchParams;
	const identity = {
		origin: url.origin,
		issuer: config.issuer,
		client: config.client,
	};
	for (const key of ["origin", "issuer", "client"] as const) {
		if (journey[key] !== identity[key]) {
			warn("oidc.mismatch");
			count("oidc", "mismatch", "/callback");
			throw new Refusal("身份确认不匹配，请返回登录重试。");
		}
	}
	if (url.pathname !== "/callback" || !valid(journey.back)) {
		warn("oidc.mismatch");
		count("oidc", "mismatch", "/callback");
		throw new Refusal("身份确认不匹配，请返回登录重试。");
	}
	const code = single(query, "code");
	if (single(query, "state") !== journey.state || !code || query.has("error")) {
		warn("oidc.mismatch");
		count("oidc", "mismatch", "/callback");
		throw new Refusal("身份确认不匹配，请返回登录重试。");
	}

	const fetcher = seat.fetcher;
	const response = await track("/api/token", "POST", "oidc", () =>
		fetcher("/api/token", {
			method: "POST",
			cache: "no-store",
			credentials: "omit",
			redirect: "error",
			signal: AbortSignal.timeout(15000),
			headers: { "content-type": "application/x-www-form-urlencoded" },
			body: new URLSearchParams({
				grant_type: "authorization_code",
				client_id: config.client,
				redirect_uri: `${url.origin}/callback`,
				code,
				code_verifier: journey.verifier,
			}),
		}),
	);
	if (!response.ok) throw new Refusal("身份确认已失效，请返回登录重试。");
	const result = await response.json();
	if (
		result.token_type !== "Bearer" ||
		typeof result.access_token !== "string" ||
		!result.access_token
	)
		throw new Refusal("暂时无法完成身份确认。");
	if (!Number.isFinite(result.expires_in) || result.expires_in <= 0)
		throw new Refusal("暂时无法完成身份确认。");
	access = {
		value: result.access_token,
		until: Date.now() + result.expires_in * 1000,
	};
	info("oidc.complete");
	return journey.back;
}
