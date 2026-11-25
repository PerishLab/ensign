import {
	type Address,
	type Client,
	connect,
	type State as Document,
	type Handle,
	type Json,
	type Reply,
	type Request,
	type Snapshot,
	type Transport,
} from "@perishlab/radicula-document";

export type { Client, Document, Handle, Json, Reply, Snapshot };
export type Scope = "app" | "shared";
export type Tone = "light" | "dark";
type Declaration = { state: "missing" } | { state: "present"; value: string };
export type Item = {
	name: string;
	options: string[];
	shared: Declaration;
	app: Declaration;
	effective: string;
	source: "default" | "shared" | "app" | "policy";
	editable: boolean;
};
export type Presentation = { system: string; tone: Tone; basis: string };
export type View = {
	app: string | null;
	items: Record<string, Item>;
	presentation?: Presentation;
};
export type Views = { shared: View; app: View };
export type Change = {
	op: "set" | "reset";
	scope: { kind: Scope; name?: string };
	name: string;
	value?: string;
};
export type Options = {
	access: () => string | undefined;
	device: () => Tone;
	fetcher?: typeof fetch;
};
export type Store = {
	client: Client<Change>;
	system: Handle<string>;
	tone: Handle<string>;
};

const base = "/folio/v1/me/documents";
const refused = new Set([400, 401, 403, 404, 422]);
const uncertain =
	"保存结果尚未确认，已暂停继续写入；请重新读取设置以确认当前状态。";

const messages: Record<number, string> = {
	409: "设置已在其他页面更新，已重新读取最新设置。",
	401: "身份确认已过期，请重新读取后重试。",
	403: "当前无法修改此设置。",
};

function describe(status: number): string {
	return (
		messages[status] ??
		(status >= 500 ? uncertain : "暂时无法完成操作，请重新读取设置后重试。")
	);
}

export class Failure extends Error {
	status: number;
	constructor(status: number, text?: string) {
		super(text ?? describe(status));
		this.status = status;
	}
}

function address(target: Address, tone: Tone): string {
	return `${target.base_rel_path}/${target.binding.key}?app=ensign&tone=${tone}`;
}

function transport(options: Options): Transport<Change> {
	const fetcher = options.fetcher ?? fetch;
	return {
		async read(target: Address): Promise<Snapshot> {
			const token = options.access();
			if (!token) throw new Failure(401);
			const response = await fetcher(address(target, options.device()), {
				method: "GET",
				cache: "no-store",
				credentials: "omit",
				redirect: "error",
				signal: AbortSignal.timeout(15000),
				headers: { authorization: `Bearer ${token}` },
			});
			if (!response.ok) {
				throw new Failure(
					response.status,
					response.status >= 500 ? "暂时无法读取设置，请重试。" : undefined,
				);
			}
			return response.json();
		},
		async write(request: Request<Change>): Promise<Reply> {
			const token = options.access();
			if (!token) return { state: "rejected", error: new Failure(401) };
			const response = await fetcher(address(request, options.device()), {
				method: "PATCH",
				cache: "no-store",
				credentials: "omit",
				redirect: "error",
				signal: AbortSignal.timeout(15000),
				headers: {
					authorization: `Bearer ${token}`,
					"content-type": "application/json",
				},
				body: JSON.stringify({
					revision: request.revision,
					changes: [request.intent],
				}),
			});
			if (response.ok) {
				return { state: "saved", snapshot: await response.json() };
			}
			if (response.status === 409) {
				return { state: "conflict", error: new Failure(409) };
			}
			if (refused.has(response.status)) {
				return { state: "rejected", error: new Failure(response.status) };
			}
			return {
				state: "unknown",
				error: new Failure(response.status, uncertain),
			};
		},
	};
}

export function open(options: Options): Store {
	const client = connect<Change>({
		base_rel_path: base,
		transport: transport(options),
		project: (body) => body,
	});
	return {
		client,
		system: client.string("/app/presentation/system", () => "google-accounts"),
		tone: client.string("/app/presentation/tone", options.device),
	};
}

export function intent(scope: Scope, name: string, value?: string): Change {
	return {
		op: value === undefined ? "reset" : "set",
		scope: { kind: scope, ...(scope === "app" ? { name: "ensign" } : {}) },
		name,
		...(value === undefined ? {} : { value }),
	};
}

function record(value: unknown): value is Record<string, unknown> {
	return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function views(body: Json | undefined): Views | undefined {
	if (!record(body)) return undefined;
	if (!record(body.app) || !record(body.shared)) return undefined;
	if (!record(body.app.items) || !record(body.shared.items)) return undefined;
	return body as unknown as Views;
}

export function notice(state: Document): string | undefined {
	if (state.phase === "unknown") {
		return state.error instanceof Failure ? state.error.message : uncertain;
	}
	if (state.error instanceof Failure) return state.error.message;
	if (state.error === undefined) return undefined;
	return state.phase === "failed"
		? "暂时无法读取设置，请重试。"
		: "暂时无法完成操作，请重新读取设置后重试。";
}
