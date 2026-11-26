export type Mood = "warn" | "hold";
export type Actor = { id: number; sub: string; login: string; name: string };
export type Rescue = { count: number; created: number | null };
export type Mint = { codes: string[] };
export type Session = {
	id: number;
	created: number;
	expires: number | null;
	current: boolean;
};
export type Token = {
	id: number;
	name: string;
	created: number;
	expires: number | null;
	current: boolean;
};
export type Inventory = { sessions: Session[]; tokens: Token[] };
export type Born = { id: number };
export type Kind =
	| "login"
	| "join"
	| "revive"
	| "profile"
	| "repass"
	| "rescue"
	| "session"
	| "logout"
	| "read";

type Spec = {
	method: "GET" | "POST" | "PATCH" | "DELETE";
	body?: unknown;
	kind: Kind;
};

const table: Partial<Record<Kind, Partial<Record<number, string>>>> = {
	login: {
		400: "请输入账号与密码。",
		401: "账号或密码不正确。",
		403: "账号或密码不正确，或账号已被停用。",
	},
	join: {
		400: "请检查填写内容，密码长度至少需要 8 位。",
		404: "邀请码无效或已被使用。",
		409: "该账号已被占用，请更换账号名后重试。",
	},
	revive: {
		400: "请检查账号、恢复码与新密码（至少 8 位）。",
		401: "账号或恢复码不正确。",
		403: "该账号已被停用，无法重置密码。",
	},
	profile: {
		400: "显示名称需为 1 至 120 个字符，且不能包含控制字符。",
		409: "资料状态已变更，请刷新后重试。",
	},
	repass: {
		400: "新密码长度至少需要 8 位，请重新输入。",
		403: "当前密码不正确。",
	},
	session: {
		404: "目标会话或令牌不存在，可能已被撤销。",
	},
};

const general: Record<number, string> = {
	400: "请求参数无效，请检查后重试。",
	401: "登录状态已失效，请重新登录。",
	403: "当前无权执行此操作。",
	404: "请求的资源不存在。",
	409: "操作与当前状态冲突，请刷新后重试。",
	429: "请求过于频繁，请稍后重试。",
};

const hold: Record<number, string> = {
	0: "无法连接服务，请检查网络后重试。",
	429: "请求过于频繁，请稍后重试。",
};

const format = new Intl.DateTimeFormat("zh-CN", {
	year: "numeric",
	month: "2-digit",
	day: "2-digit",
	hour: "2-digit",
	minute: "2-digit",
	hour12: false,
});

export function explain(
	status: number,
	kind: Kind = "read",
): { text: string; mood: Mood } {
	const paused = hold[status];
	if (paused !== undefined) return { text: paused, mood: "hold" };
	if (status >= 500) {
		return { text: "服务暂时不可用，请稍后重试。", mood: "hold" };
	}
	const text =
		table[kind]?.[status] ??
		general[status] ??
		"暂时无法完成请求，请稍后重试。";
	return { text, mood: "warn" };
}

export class Fault extends Error {
	status: number;
	mood: Mood;
	code: string;

	constructor(status: number, kind: Kind = "read", text?: string, mood?: Mood) {
		const info = explain(status, kind);
		super(text ?? info.text);
		this.status = status;
		this.mood = mood ?? info.mood;
		this.code = status === 0 ? "NET" : String(status);
	}
}

export function validpass(pass: string): boolean {
	return Array.from(pass).length >= 8;
}

export function validname(name: string): boolean {
	for (const char of name) {
		const point = char.codePointAt(0) ?? 0;
		if (point < 32 || (point >= 127 && point <= 159)) return false;
	}
	const count = Array.from(name.trim()).length;
	return count >= 1 && count <= 120;
}

export function stamp(seconds: number | null, empty = "永不过期"): string {
	if (seconds === null || !Number.isFinite(seconds)) return empty;
	return format.format(new Date(seconds * 1000)).replaceAll("/", "-");
}

export class Account {
	fetcher: typeof fetch;

	constructor(fetcher: typeof fetch = fetch) {
		this.fetcher = fetcher;
	}

	async call(path: string, spec: Spec): Promise<Response> {
		const init: RequestInit = {
			method: spec.method,
			credentials: "same-origin",
			cache: "no-store",
			redirect: "error",
			signal: AbortSignal.timeout(15000),
		};
		if (spec.body !== undefined) {
			init.headers = { "Content-Type": "application/json" };
			init.body = JSON.stringify(spec.body);
		}
		const fetcher = this.fetcher;
		try {
			return await fetcher(path, init);
		} catch {
			throw new Fault(0, spec.kind);
		}
	}

	async parse<T>(response: Response, kind: Kind): Promise<T> {
		try {
			return (await response.json()) as T;
		} catch {
			throw new Fault(
				response.status,
				kind,
				"服务返回了无法解析的响应，请稍后重试。",
				"hold",
			);
		}
	}

	async whoami(): Promise<Actor> {
		const response = await this.call("/api/whoami", {
			method: "GET",
			kind: "read",
		});
		if (response.status !== 200) throw new Fault(response.status, "read");
		return this.parse<Actor>(response, "read");
	}

	async login(fields: { login: string; pass: string }): Promise<Born> {
		const response = await this.call("/api/login", {
			method: "POST",
			body: fields,
			kind: "login",
		});
		if (response.status !== 201) throw new Fault(response.status, "login");
		return this.parse<Born>(response, "login");
	}

	async logout(): Promise<void> {
		const response = await this.call("/api/logout", {
			method: "POST",
			body: {},
			kind: "logout",
		});
		if ([204, 400, 401].includes(response.status)) return;
		throw new Fault(response.status, "logout");
	}

	async join(fields: {
		code: string;
		login: string;
		name: string;
		pass: string;
	}): Promise<Born> {
		const response = await this.call("/api/join", {
			method: "POST",
			body: fields,
			kind: "join",
		});
		if (response.status !== 201) throw new Fault(response.status, "join");
		return this.parse<Born>(response, "join");
	}

	async revive(fields: {
		login: string;
		code: string;
		pass: string;
	}): Promise<void> {
		const response = await this.call("/api/revive", {
			method: "POST",
			body: fields,
			kind: "revive",
		});
		if (response.status !== 204) throw new Fault(response.status, "revive");
	}

	async rename(name: string): Promise<Actor> {
		const response = await this.call("/api/profile", {
			method: "PATCH",
			body: { name },
			kind: "profile",
		});
		if (response.status !== 200) throw new Fault(response.status, "profile");
		return this.parse<Actor>(response, "profile");
	}

	async repass(fields: { old: string; pass: string }): Promise<void> {
		const response = await this.call("/api/repass", {
			method: "POST",
			body: fields,
			kind: "repass",
		});
		if (response.status !== 204) throw new Fault(response.status, "repass");
	}

	async rescue(): Promise<Rescue> {
		const response = await this.call("/api/rescue", {
			method: "GET",
			kind: "rescue",
		});
		if (response.status !== 200) throw new Fault(response.status, "rescue");
		return this.parse<Rescue>(response, "rescue");
	}

	async mint(): Promise<Mint> {
		const response = await this.call("/api/mint", {
			method: "POST",
			kind: "rescue",
		});
		if (response.status !== 201) throw new Fault(response.status, "rescue");
		return this.parse<Mint>(response, "rescue");
	}

	async sessions(): Promise<Inventory> {
		const response = await this.call("/api/sessions", {
			method: "GET",
			kind: "session",
		});
		if (response.status !== 200) throw new Fault(response.status, "session");
		return this.parse<Inventory>(response, "session");
	}

	async evict(id: number): Promise<void> {
		const response = await this.call(`/api/sessions/${id}`, {
			method: "DELETE",
			kind: "session",
		});
		if (response.status !== 204) throw new Fault(response.status, "session");
	}

	async revoke(id: number): Promise<void> {
		const response = await this.call(`/api/tokens/${id}`, {
			method: "DELETE",
			kind: "session",
		});
		if (response.status !== 204) throw new Fault(response.status, "session");
	}
}

export const account = new Account();
