import { Fault, validname } from "./account.ts";
import {
	type Action,
	describe,
	fail,
	type Mode,
	validapp,
	validnote,
	validorigin,
	validredirect,
	validslug,
} from "./rules.ts";

export type Entry = {
	id: number;
	sub: string;
	login: string | null;
	name: string | null;
	kind: string;
	barred: boolean;
	created: number;
	teams: string[];
};

export type Invite = {
	id: number;
	note: string;
	created: number;
	expires: number | null;
};

export type Issued = Invite & { code: string };

export type Member = {
	tie: number;
	id: number;
	sub: string;
	login: string | null;
	name: string | null;
	kind: string;
	barred: boolean;
};

export type Team = {
	id: number;
	name: string;
	created: number;
	members: Member[];
};

export type Tie = { tie: number; team: number; actor: number };

export type App = {
	id: number;
	name: string;
	slug: string;
	home: string;
	redirect: string;
	mode: Mode;
	barred: boolean;
	created: number;
};

export type Card = { note?: string; ttl?: number };

export type Craft = {
	name: string;
	slug: string;
	mode: Mode;
	home?: string;
	redirect?: string;
};

export type Amend = {
	name?: string;
	home?: string;
	redirect?: string;
	barred?: boolean;
};

type Spec = {
	method: "GET" | "POST" | "PATCH" | "DELETE";
	body?: unknown;
	action: Action;
};

export class Admin {
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
			throw fail(0, spec.action);
		}
	}

	async parse<T>(response: Response): Promise<T> {
		try {
			return (await response.json()) as T;
		} catch {
			throw new Fault(
				response.status,
				"read",
				"服务返回了无法解析的响应，请刷新列表核对当前状态。",
				"hold",
			);
		}
	}

	async actors(): Promise<Entry[]> {
		const response = await this.call("/api/admin/actors", {
			method: "GET",
			action: "read",
		});
		if (response.status !== 200) throw fail(response.status, "read");
		return this.parse<Entry[]>(response);
	}

	async actor(id: number): Promise<Entry> {
		const response = await this.call(`/api/admin/actors/${id}`, {
			method: "GET",
			action: "read",
		});
		if (response.status !== 200) throw fail(response.status, "read");
		return this.parse<Entry>(response);
	}

	async bar(id: number, barred: boolean): Promise<Entry> {
		const response = await this.call(`/api/admin/actors/${id}`, {
			method: "PATCH",
			body: { barred },
			action: "bar",
		});
		if (response.status !== 200) throw fail(response.status, "bar");
		return this.parse<Entry>(response);
	}

	async invites(): Promise<Invite[]> {
		const response = await this.call("/api/admin/invites", {
			method: "GET",
			action: "read",
		});
		if (response.status !== 200) throw fail(response.status, "read");
		return this.parse<Invite[]>(response);
	}

	async invite(payload: Card = {}): Promise<Issued> {
		const body: Card = {};
		if (payload.note !== undefined) body.note = payload.note;
		if (payload.ttl !== undefined) body.ttl = payload.ttl;
		const response = await this.call("/api/admin/invites", {
			method: "POST",
			body,
			action: "invite",
		});
		if (response.status !== 201) throw fail(response.status, "invite");
		return this.parse<Issued>(response);
	}

	async cancel(id: number): Promise<void> {
		const response = await this.call(`/api/admin/invites/${id}`, {
			method: "DELETE",
			action: "cancel",
		});
		if (response.status !== 204) throw fail(response.status, "cancel");
	}

	async teams(): Promise<Team[]> {
		const response = await this.call("/api/admin/teams", {
			method: "GET",
			action: "read",
		});
		if (response.status !== 200) throw fail(response.status, "read");
		return this.parse<Team[]>(response);
	}

	async team(name: string): Promise<Team> {
		const response = await this.call("/api/admin/teams", {
			method: "POST",
			body: { name },
			action: "team",
		});
		if (response.status !== 201) throw fail(response.status, "team");
		return this.parse<Team>(response);
	}

	async enroll(team: number, actor: number): Promise<Tie> {
		const response = await this.call(`/api/admin/teams/${team}/members`, {
			method: "POST",
			body: { actor },
			action: "enroll",
		});
		if (response.status !== 201) throw fail(response.status, "enroll");
		return this.parse<Tie>(response);
	}

	async eject(team: number, tie: number): Promise<void> {
		const response = await this.call(
			`/api/admin/teams/${team}/members/${tie}`,
			{
				method: "DELETE",
				action: "eject",
			},
		);
		if (response.status !== 204) throw fail(response.status, "eject");
	}

	async apps(): Promise<App[]> {
		const response = await this.call("/api/admin/apps", {
			method: "GET",
			action: "read",
		});
		if (response.status !== 200) throw fail(response.status, "read");
		return this.parse<App[]>(response);
	}

	async create(payload: Craft): Promise<App> {
		const body: Craft = {
			name: payload.name,
			slug: payload.slug,
			mode: payload.mode,
		};
		if (payload.home !== undefined) body.home = payload.home;
		if (payload.redirect !== undefined) body.redirect = payload.redirect;
		const response = await this.call("/api/admin/apps", {
			method: "POST",
			body,
			action: "create",
		});
		if (response.status !== 201) throw fail(response.status, "create");
		return this.parse<App>(response);
	}

	async update(id: number, payload: Amend): Promise<App> {
		const body: Amend = {};
		if (payload.name !== undefined) body.name = payload.name;
		if (payload.home !== undefined) body.home = payload.home;
		if (payload.redirect !== undefined) body.redirect = payload.redirect;
		if (payload.barred !== undefined) body.barred = payload.barred;
		const response = await this.call(`/api/admin/apps/${id}`, {
			method: "PATCH",
			body,
			action: "update",
		});
		if (response.status !== 200) throw fail(response.status, "update");
		return this.parse<App>(response);
	}
}

export const admin = new Admin();
export {
	type Action,
	describe,
	fail,
	type Mode,
	validapp,
	validname,
	validnote,
	validorigin,
	validredirect,
	validslug,
};
