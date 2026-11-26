export class Refusal extends Error {}

export type Config = { issuer: string; client: string };
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

export function held(): string | undefined {
	return access && access.until > Date.now() ? access.value : undefined;
}

export function forget(): void {
	access = undefined;
	globalThis.sessionStorage?.removeItem(slot);
}

export async function configuration(
	fetcher: typeof fetch = fetch,
): Promise<Config | undefined> {
	const response = await fetcher("/client.json", {
		cache: "no-store",
		credentials: "omit",
		redirect: "error",
		signal: AbortSignal.timeout(15000),
	});
	if (!response.ok) throw new Refusal("暂时无法读取设置，请重试。");
	const config = await response.json();
	if (!config.issuer && !config.client) return undefined;
	if (
		typeof config.issuer !== "string" ||
		typeof config.client !== "string" ||
		!config.client
	)
		throw new Refusal("暂时无法确认设置身份，请重试。");
	const discovery = await fetcher("/api/.well-known/openid-configuration", {
		cache: "no-store",
		credentials: "omit",
		redirect: "error",
		signal: AbortSignal.timeout(15000),
	});
	if (!discovery.ok) throw new Refusal("暂时无法确认设置身份，请重试。");
	const known = await discovery.json();
	if (
		known.issuer !== config.issuer ||
		known.authorization_endpoint !== `${config.issuer}/authorize` ||
		known.token_endpoint !== `${config.issuer}/token`
	)
		throw new Refusal("无法确认设置服务的身份。");
	return { issuer: config.issuer, client: config.client };
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
	if (!raw) throw new Refusal("身份确认已失效，请返回登录重试。");
	const journey: Journey = JSON.parse(raw);
	const url = new URL(seat.url);
	const query = url.searchParams;
	const identity = {
		origin: url.origin,
		issuer: config.issuer,
		client: config.client,
	};
	for (const key of ["origin", "issuer", "client"] as const) {
		if (journey[key] !== identity[key])
			throw new Refusal("身份确认不匹配，请返回登录重试。");
	}
	if (url.pathname !== "/callback" || !valid(journey.back))
		throw new Refusal("身份确认不匹配，请返回登录重试。");
	const code = single(query, "code");
	if (single(query, "state") !== journey.state || !code || query.has("error"))
		throw new Refusal("身份确认不匹配，请返回登录重试。");

	const fetcher = seat.fetcher;
	const response = await fetcher("/api/token", {
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
	});
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
	return journey.back;
}
