type Row = { id: number } & Record<string, unknown>;

type Won = "ok" | "miss" | "barred" | "fail";

type Made = "ok" | "floor" | "code" | "taken" | "fail";

export function enter(login: string, pass: string): Promise<Won> {
	return send("/login", { login, pass })
		.then((res): Won => {
			if (res.ok) {
				return "ok";
			}
			if (res.status === 403) {
				return "barred";
			}
			return res.status === 401 || res.status === 400 ? "miss" : "fail";
		})
		.catch((): Won => "fail");
}

export function join(
	code: string,
	login: string,
	name: string,
	pass: string,
): Promise<Made> {
	return send("/join", { code, login, name, pass })
		.then((res): Made => {
			if (res.ok) {
				return "ok";
			}
			if (res.status === 400) {
				return "floor";
			}
			if (res.status === 404) {
				return "code";
			}
			return res.status === 409 ? "taken" : "fail";
		})
		.catch((): Made => "fail");
}

type Rescued = "ok" | "miss" | "barred" | "floor" | "fail";

export function revive(
	login: string,
	code: string,
	pass: string,
): Promise<Rescued> {
	return send("/revive", { login, code, pass })
		.then((res): Rescued => {
			if (res.ok) {
				return "ok";
			}
			if (res.status === 403) {
				return "barred";
			}
			if (res.status === 400) {
				return "floor";
			}
			return res.status === 401 ? "miss" : "fail";
		})
		.catch((): Rescued => "fail");
}

type Shift = "ok" | "floor" | "miss" | "anon" | "fail";

export function repass(old: string, pass: string): Promise<Shift> {
	return send("/repass", { old, pass })
		.then((res): Shift => {
			if (res.ok) {
				return "ok";
			}
			if (res.status === 400) {
				return "floor";
			}
			if (res.status === 403) {
				return "miss";
			}
			return res.status === 401 ? "anon" : "fail";
		})
		.catch((): Shift => "fail");
}

export function leave(): Promise<boolean> {
	return fetch("/logout", { method: "POST" })
		.then((res) => res.ok || res.status === 400 || res.status === 404)
		.catch(() => false);
}

type Me = { id: number; login: string; name: string };

export function whoami(): Promise<Me | "anon" | "fail"> {
	return fetch("/whoami")
		.then((res): Promise<Me | "anon" | "fail"> => {
			if (res.ok) {
				return res.json() as Promise<Me>;
			}
			return Promise.resolve(res.status === 401 ? "anon" : "fail");
		})
		.catch(() => "fail");
}

export function mint(): Promise<string[] | "anon" | "fail"> {
	return fetch("/mint", { method: "POST" })
		.then((res): Promise<string[] | "anon" | "fail"> => {
			if (res.ok) {
				return res.json().then((bag: { codes: string[] }) => bag.codes);
			}
			return Promise.resolve(res.status === 401 ? "anon" : "fail");
		})
		.catch(() => "fail");
}

type Opened = { code: string; shut: "" | "anon" | "denied" | "fail" };

export function invite(note: string): Promise<Opened> {
	return send("/invite", { note })
		.then((res): Promise<Opened> => {
			if (res.ok) {
				return res
					.json()
					.then((bag: { code: string }) => ({ code: bag.code, shut: "" }));
			}
			if (res.status === 401) {
				return Promise.resolve({ code: "", shut: "anon" });
			}
			const shut = res.status === 403 ? "denied" : "fail";
			return Promise.resolve({ code: "", shut });
		})
		.catch((): Opened => ({ code: "", shut: "fail" }));
}

export function rows(unit: string): Promise<Row[] | null> {
	return fetch(`/${unit}`)
		.then((res): Promise<Row[] | null> => {
			return res.ok ? (res.json() as Promise<Row[]>) : Promise.resolve(null);
		})
		.catch(() => null);
}

type Put = "ok" | "anon" | "denied" | "clash" | "fail";

export function put(unit: string, body: Record<string, string>): Promise<Put> {
	return send(`/${unit}`, body)
		.then((res): Put => {
			if (res.ok) {
				return "ok";
			}
			if (res.status === 401) {
				return "anon";
			}
			if (res.status === 403 || res.status === 404) {
				return "denied";
			}
			return res.status === 409 ? "clash" : "fail";
		})
		.catch((): Put => "fail");
}

type Set = "ok" | "anon" | "denied" | "fail";

export function set(
	unit: string,
	id: number,
	body: Record<string, string>,
): Promise<Set> {
	return fetch(`/${unit}/${id}`, {
		method: "PATCH",
		headers: { "content-type": "application/json" },
		body: JSON.stringify(body),
	})
		.then((res): Set => {
			if (res.ok) {
				return "ok";
			}
			if (res.status === 401) {
				return "anon";
			}
			return res.status === 403 || res.status === 404 ? "denied" : "fail";
		})
		.catch((): Set => "fail");
}

export function crews(me: number): Promise<string[] | null> {
	return fetch("/query", {
		method: "POST",
		headers: { "content-type": "application/json" },
		body: JSON.stringify({ q: `from Team where members has "${me}"` }),
	})
		.then((res): Promise<string[] | null> => {
			if (!res.ok) {
				return Promise.resolve(null);
			}
			return res
				.json()
				.then((pack: { root: string; bags: Record<string, Row[]> }) => {
					const held = pack.bags[pack.root] ?? [];
					return held.map((row) => String(row.name ?? ""));
				});
		})
		.catch(() => null);
}

function send(path: string, body: Record<string, string>): Promise<Response> {
	return fetch(path, {
		method: "POST",
		headers: { "content-type": "application/json" },
		body: JSON.stringify(body),
	});
}
