import { Fault, type Mood } from "./account.ts";

export type Mode = "oidc" | "forward" | "resource";

export type Action =
	| "read"
	| "bar"
	| "invite"
	| "cancel"
	| "team"
	| "enroll"
	| "eject"
	| "create"
	| "update";

const table: Partial<Record<Action, Partial<Record<number, string>>>> = {
	bar: {
		400: "不能停用当前登录的管理员账号自身。",
		404: "目标账号不存在或不可见。",
		409: "无法停用系统保留的 ensign 服务账号，请刷新确认当前状态。",
	},
	invite: {
		400: "邀请参数无效：备注最多 120 个字符且不含控制字符，有效期需在 1 秒至 30 天内。",
	},
	cancel: {
		404: "该邀请已不存在，可能已被使用或撤销。",
	},
	team: {
		400: "团队名称需为 1 至 120 个字符，且不能包含控制字符。",
		409: "团队名称已存在或状态冲突，请刷新列表后核对。",
	},
	enroll: {
		404: "目标团队或账号不存在，请刷新后重试。",
		409: "该账号已在团队中或状态已变更，请刷新列表核对。",
	},
	eject: {
		404: "该成员关系已不存在，可能已被移除。",
	},
	create: {
		400: "应用参数无效，请检查名称、标识、主页地址及回调地址格式。",
		409: "应用标识 (slug) 已被占用，请更换标识或刷新列表核对。",
	},
	update: {
		400: "更新参数无效，请检查名称、主页地址或回调地址格式。",
		404: "目标应用不存在，请刷新列表。",
	},
};

const general: Record<number, string> = {
	400: "请求参数无效，请检查后重试。",
	401: "登录状态已失效，请重新登录。",
	403: "当前账号缺少该管理操作权限（Keel 拒绝）。",
	404: "请求的资源不存在。",
	409: "操作与当前状态冲突，请刷新后检查。",
	429: "请求过于频繁，请稍后重试。",
};

const paused: Record<number, [string, string]> = {
	0: [
		"无法连接服务，请检查网络后重试。",
		"网络连接中断，操作结果尚不明确，请刷新列表核对实际状态。",
	],
	429: [general[429], general[429]],
};

const pattern = /^[a-z0-9](?:[a-z0-9-]{0,62}[a-z0-9])?$/;

export function describe(
	status: number,
	action: Action = "read",
): { text: string; mood: Mood } {
	const write = action === "read" ? 0 : 1;
	const hold = paused[status];
	if (hold !== undefined) return { text: hold[write], mood: "hold" };
	if (status >= 500) {
		const text =
			write === 1
				? "服务暂时不可用，操作结果尚不明确，请刷新列表核对实际状态。"
				: "服务暂时不可用，请稍后重试。";
		return { text, mood: "hold" };
	}
	const text =
		table[action]?.[status] ??
		general[status] ??
		"暂时无法完成请求，请稍后重试。";
	return { text, mood: status === 409 ? "hold" : "warn" };
}

export function fail(status: number, action: Action): Fault {
	const info = describe(status, action);
	return new Fault(status, "read", info.text, info.mood);
}

export function validnote(note: string): boolean {
	for (const char of note) {
		const point = char.codePointAt(0) ?? 0;
		if (point < 32 || (point >= 127 && point <= 159)) return false;
	}
	return Array.from(note).length <= 120;
}

export function validslug(slug: string): boolean {
	return pattern.test(slug);
}

function scheme(home: string): boolean {
	return home.startsWith("https://") || home.startsWith("http://");
}

function authority(home: string): string {
	const rest = home.slice(home.indexOf("://") + 3);
	const slash = rest.indexOf("/");
	const query = rest.indexOf("?");
	let end = rest.length;
	if (slash !== -1 && slash < end) end = slash;
	if (query !== -1 && query < end) end = query;
	return rest.slice(0, end);
}

export function validorigin(home: string): boolean {
	if (!home || !scheme(home)) return false;
	if (home.includes("#") || home.includes("\\") || /\s/u.test(home)) {
		return false;
	}
	for (const char of home) {
		const point = char.codePointAt(0) ?? 0;
		if (point < 32 || (point >= 127 && point <= 159)) return false;
	}
	if (authority(home).includes("@")) return false;
	let url: URL;
	try {
		url = new URL(home);
	} catch {
		return false;
	}
	if (url.username || url.password || !url.hostname) return false;
	if (url.protocol === "https:") return true;
	return (
		url.protocol === "http:" &&
		(url.hostname === "localhost" || url.hostname === "127.0.0.1")
	);
}

export function validredirect(redirect: string): boolean {
	return validorigin(redirect) && !redirect.includes("?");
}

export function validapp(
	mode: Mode,
	home: string,
	redirect: string,
): string | undefined {
	const needhome = mode === "oidc" || mode === "forward";
	if (needhome && !home) return "当前模式必须填写有效的主页地址。";
	if (home && !validorigin(home)) {
		return "主页地址须为 https://（或本地 http://localhost / http://127.0.0.1），且不含空格与 #。";
	}
	if (mode === "oidc" && !redirect) return "OIDC 应用必须填写有效的回调地址。";
	if (redirect && !validredirect(redirect)) {
		return "回调地址须为 https://（或本地 localhost/127.0.0.1），且不含查询参数与 #。";
	}
	return undefined;
}
