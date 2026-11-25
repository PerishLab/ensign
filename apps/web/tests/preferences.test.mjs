import assert from "node:assert/strict";
import test from "node:test";
import { Failure, intent, notice, open, views } from "../preferences.ts";

function item(name, effective, source = "default", editable = true) {
	const options =
		name === "theme.tone"
			? ["system", "light", "dark"]
			: ["google-accounts", "material"];
	const missing = { state: "missing" };
	return {
		name,
		options,
		shared: missing,
		app: missing,
		effective,
		source,
		editable,
	};
}

function snapshot(revision, system = "google-accounts", tone = "light") {
	return {
		revision,
		document: {
			shared: {
				app: null,
				items: {
					"theme.family": item("theme.family", system, "shared"),
					"theme.tone": item("theme.tone", tone, "policy", false),
				},
			},
			app: {
				app: "ensign",
				presentation: { system, tone, basis: "app" },
				items: {
					"theme.family": item("theme.family", system, "app"),
					"theme.tone": item("theme.tone", tone, "app"),
				},
			},
		},
	};
}

const json = (body, status = 200) => Response.json(body, { status });
const store = (fetcher, device = () => "light") =>
	open({ access: () => "held", device, fetcher });
const binding = (identity) => ({ key: "preferences", identity });
const settled = (state) => state.phase === "ready" && state.pending === 0;

function until(client, check) {
	if (check(client.state)) return Promise.resolve(client.state);
	return new Promise((resolve) => {
		const stop = client.subscribe(() => {
			if (!check(client.state)) return;
			stop();
			resolve(client.state);
		});
	});
}

test("reset is absence, empty string remains explicit and application scope stays qualified", () => {
	assert.deepEqual(intent("shared", "theme.tone"), {
		op: "reset",
		scope: { kind: "shared" },
		name: "theme.tone",
	});
	assert.deepEqual(intent("app", "theme.tone", ""), {
		op: "set",
		scope: { kind: "app", name: "ensign" },
		name: "theme.tone",
		value: "",
	});
});

test("single-request read fetches both views from folio documents endpoint", async () => {
	const calls = [];
	const held = store(
		async (url, init) => {
			calls.push({ url, init });
			return json(snapshot("r1", "material", "light"));
		},
		() => "dark",
	);
	await held.client.select(binding("u1:dark:0"));
	assert.equal(calls.length, 1);
	assert.equal(
		calls[0].url,
		"/folio/v1/me/documents/preferences?app=ensign&tone=dark",
	);
	assert.equal(calls[0].init.method, "GET");
	assert.equal(calls[0].init.credentials, "omit");
	assert.equal(calls[0].init.cache, "no-store");
	assert.equal(calls[0].init.redirect, "error");
	assert.ok(calls[0].init.signal instanceof AbortSignal);
	assert.equal(calls[0].init.headers.authorization, "Bearer held");
	const body = views(held.client.state.document);
	assert.equal(body?.app.presentation?.system, "material");
	assert.equal(body?.app.presentation?.tone, "light");
	assert.equal(body?.shared.items["theme.tone"].source, "policy");
	assert.equal(body?.shared.items["theme.tone"].editable, false);
});

test("single-request confirmed write uses PATCH snapshot directly without follow-up GET", async () => {
	const calls = [];
	const held = store(
		async (url, init) => {
			calls.push({ url, init });
			const next =
				init.method === "GET"
					? snapshot("r1", "google-accounts", "light")
					: snapshot("r2", "material", "dark");
			return json(next);
		},
		() => "dark",
	);
	await held.client.select(binding("u1:dark:0"));
	const change = intent("app", "theme.tone", "dark");
	held.client.update(change);
	const state = await until(held.client, settled);
	assert.equal(calls.length, 2);
	assert.equal(calls[0].init.method, "GET");
	assert.equal(calls[1].init.method, "PATCH");
	assert.ok(calls[1].init.signal instanceof AbortSignal);
	assert.equal(calls[1].init.headers["content-type"], "application/json");
	assert.deepEqual(JSON.parse(calls[1].init.body), {
		revision: "r1",
		changes: [change],
	});
	assert.equal(state.confirmed?.revision, "r2");
	assert.equal(state.error, undefined);
	assert.equal(held.system.read().value, "material");
	assert.equal(held.tone.read().value, "dark");
});

test("409 conflict reloads fresh snapshot, retains conflict error, and lets next write succeed", async () => {
	const calls = [];
	let reads = 0;
	let writes = 0;
	const held = store(async (url, init) => {
		calls.push({ url, init });
		if (init.method === "GET") {
			reads += 1;
			return json(
				reads === 1 ? snapshot("r1") : snapshot("r2", "material", "dark"),
			);
		}
		writes += 1;
		if (writes === 1) return json({ error: "c" }, 409);
		return json(snapshot("r3", "material", "light"));
	});
	await held.client.select(binding("u1:light:0"));
	held.client.update(intent("app", "theme.tone", "dark"));
	const conflicted = await until(
		held.client,
		(next) => settled(next) && next.confirmed?.revision === "r2",
	);
	assert.ok(conflicted.error instanceof Failure);
	assert.equal(conflicted.error.status, 409);
	assert.equal(
		notice(conflicted),
		"设置已在其他页面更新，已重新读取最新设置。",
	);
	held.client.update(intent("app", "theme.tone", "light"));
	const saved = await until(
		held.client,
		(next) => settled(next) && next.confirmed?.revision === "r3",
	);
	assert.equal(saved.error, undefined);
	assert.equal(JSON.parse(calls[3].init.body).revision, "r2");
});

test("definite refusals 400, 401, 403, 404 and 422 roll back intent and preserve snapshot", async () => {
	for (const status of [400, 401, 403, 404, 422]) {
		const held = store(async (_url, init) =>
			init.method === "GET"
				? json(snapshot("r1"))
				: json({ error: "x" }, status),
		);
		await held.client.select(binding(`u:${status}`));
		held.client.update(intent("shared", "theme.tone", "dark"));
		const state = await until(
			held.client,
			(next) => settled(next) && next.error !== undefined,
		);
		assert.equal(state.confirmed?.revision, "r1");
		assert.ok(state.error instanceof Failure);
		assert.equal(state.error.status, status);
		assert.equal(notice(state), state.error.message);
	}
});

test("uncertain write on 503, 429 or thrown network error freezes queue until new epoch", async () => {
	for (const kind of [503, 429, "throw"]) {
		let writes = 0;
		let reads = 0;
		const held = store(async (_url, init) => {
			if (init.method === "GET") {
				reads += 1;
				return json(snapshot(`r${reads}`));
			}
			writes += 1;
			if (typeof kind === "number") return json({ error: "down" }, kind);
			throw new TypeError("network failed");
		});
		await held.client.select(binding("u1:light:0"));
		held.client.update(intent("app", "theme.tone", "dark"));
		const paused = await until(held.client, (next) => next.phase === "unknown");
		assert.equal(paused.pending, 1);
		assert.equal(writes, 1);
		assert.equal(
			notice(paused),
			"保存结果尚未确认，已暂停继续写入；请重新读取设置以确认当前状态。",
		);
		assert.throws(() =>
			held.client.update(intent("app", "theme.tone", "light")),
		);
		await held.client.refresh();
		await held.client.select(binding("u1:light:0"));
		assert.equal(held.client.state.phase, "unknown");
		assert.equal(reads, 1);
		await held.client.select(binding("u1:light:1"));
		assert.equal(held.client.state.phase, "ready");
		assert.equal(held.client.state.pending, 0);
		assert.equal(held.client.state.confirmed?.revision, "r2");
	}
});

test("handles apply lazy defaults when signed out or missing and avoid defaults on failed or unknown", async () => {
	let mode = "light";
	let calls = 0;
	let fail = false;
	const empty = {
		revision: "r1",
		document: {
			shared: { app: null, items: {} },
			app: { app: "ensign", items: {} },
		},
	};
	const held = store(
		async (_url, init) =>
			init.method !== "GET" || fail
				? json({ error: "down" }, 503)
				: json(empty),
		() => {
			calls += 1;
			return mode;
		},
	);
	assert.deepEqual(held.system.read(), {
		state: "value",
		source: "default",
		value: "google-accounts",
	});
	assert.deepEqual(held.tone.read(), {
		state: "value",
		source: "default",
		value: "light",
	});
	mode = "dark";
	assert.equal(held.tone.read().value, "dark");
	await held.client.select(binding("u1:dark:0"));
	assert.equal(held.system.read().source, "default");
	assert.equal(held.tone.read().value, "dark");
	held.client.update(intent("app", "theme.tone", "light"));
	await until(held.client, (next) => next.phase === "unknown");
	const paused = calls;
	assert.equal(held.system.read().state, "unknown");
	assert.equal(held.tone.read().state, "unknown");
	assert.equal(calls, paused);
	fail = true;
	await held.client.select(binding("u1:dark:1"));
	const failed = calls;
	assert.equal(held.system.read().state, "failed");
	assert.equal(held.tone.read().state, "failed");
	assert.equal(calls, failed);
});

test("identity switch discards late responses and select(null) returns to signed-out", async () => {
	const pending = [];
	const held = store(() => new Promise((resolve) => pending.push(resolve)));
	const first = held.client.select(binding("u1:light:0"));
	const second = held.client.select(binding("u2:light:0"));
	pending[1](json(snapshot("r2", "material", "dark")));
	await second;
	assert.equal(held.client.state.confirmed?.revision, "r2");
	pending[0](json(snapshot("r1", "google-accounts", "light")));
	await first;
	assert.equal(held.client.state.confirmed?.revision, "r2");
	assert.equal(held.system.read().value, "material");
	await held.client.select(null);
	assert.equal(held.client.state.phase, "signed-out");
	assert.equal(held.client.state.confirmed, undefined);
});
