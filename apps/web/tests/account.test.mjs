import assert from "node:assert/strict";
import test from "node:test";
import {
	Account,
	explain,
	Fault,
	stamp,
	validname,
	validpass,
} from "../account.ts";

function mock(handler) {
	const calls = [];
	const fetcher = async function (url, init) {
		assert.equal(this, undefined);
		calls.push({ url, init });
		return handler(url, init);
	};
	return { api: new Account(fetcher), calls };
}

test("validators enforce password length and display name rules", () => {
	assert.equal(validpass("1234567"), false);
	assert.equal(validpass("12345678"), true);
	assert.equal(validpass("你好世界密码八位"), true);
	assert.equal(validname(""), false);
	assert.equal(validname("   "), false);
	assert.equal(validname("Ada\nLovelace"), false);
	assert.equal(validname("Ada\u007f"), false);
	assert.equal(validname("a".repeat(121)), false);
	assert.equal(validname("  Ada Lovelace  "), true);
	assert.equal(validname("a".repeat(120)), true);
});

test("stamp converts Unix seconds to readable date and handles null", () => {
	assert.equal(stamp(null), "永不过期");
	assert.equal(stamp(null, "—"), "—");
	assert.equal(stamp(Number.NaN, "—"), "—");
	const formatted = stamp(1700000000);
	assert.match(formatted, /^\d{4}-\d{2}-\d{2} \d{2}:\d{2}$/);
});

test("every endpoint enforces same-origin, no-store, error redirect, and timeout signal", async () => {
	const replies = {
		"/api/whoami": () =>
			Response.json({ id: 1, sub: "s1", login: "ada", name: "Ada" }),
		"/api/login": () => Response.json({ id: 1 }, { status: 201 }),
		"/api/join": () => Response.json({ id: 1 }, { status: 201 }),
		"/api/profile": () =>
			Response.json({ id: 1, sub: "s1", login: "ada", name: "Ada Byron" }),
		"/api/rescue": () => Response.json({ count: 3, created: 1700000000 }),
		"/api/mint": () =>
			Response.json({ codes: ["a", "b", "c"] }, { status: 201 }),
		"/api/sessions": () => Response.json({ sessions: [], tokens: [] }),
	};
	const { api, calls } = mock(
		(url) => replies[url]?.() ?? new Response(null, { status: 204 }),
	);

	assert.equal((await api.whoami()).login, "ada");
	assert.equal((await api.login({ login: "ada", pass: "secret123" })).id, 1);
	await api.logout();
	assert.equal(
		(
			await api.join({
				code: "inv",
				login: "ada",
				name: "Ada",
				pass: "secret123",
			})
		).id,
		1,
	);
	await api.revive({ login: "ada", code: "res", pass: "newsecret" });
	assert.equal((await api.rename("Ada Byron")).name, "Ada Byron");
	await api.repass({ old: "oldsecret", pass: "newsecret" });
	assert.equal((await api.rescue()).count, 3);
	assert.deepEqual((await api.mint()).codes, ["a", "b", "c"]);
	assert.deepEqual(await api.sessions(), { sessions: [], tokens: [] });
	await api.evict(42);
	await api.revoke(99);

	assert.equal(calls.length, 12);
	for (const entry of calls) {
		assert.equal(entry.init.credentials, "same-origin");
		assert.equal(entry.init.cache, "no-store");
		assert.equal(entry.init.redirect, "error");
		assert.ok(entry.init.signal instanceof AbortSignal);
	}
	assert.equal(calls[5].init.method, "PATCH");
	assert.deepEqual(JSON.parse(calls[5].init.body), { name: "Ada Byron" });
	assert.equal(calls[10].url, "/api/sessions/42");
	assert.equal(calls[10].init.method, "DELETE");
	assert.equal(calls[11].url, "/api/tokens/99");
	assert.equal(calls[11].init.method, "DELETE");
});

test("logout treats 204, 400 and 401 as signed out and surfaces 503 as hold", async () => {
	for (const status of [204, 400, 401]) {
		const { api } = mock(() => new Response(null, { status }));
		await api.logout();
	}
	const { api } = mock(() => new Response(null, { status: 503 }));
	await assert.rejects(
		() => api.logout(),
		(err) => err instanceof Fault && err.status === 503 && err.mood === "hold",
	);
});

test("malformed JSON on 200 or 201 throws a hold Fault instead of SyntaxError", async () => {
	const broken = () =>
		new Response("not-json", {
			status: 200,
			headers: { "content-type": "text/plain" },
		});
	const { api } = mock(broken);
	let caught;
	try {
		await api.whoami();
	} catch (err) {
		caught = err;
	}
	assert.ok(caught instanceof Fault);
	assert.equal(caught.status, 200);
	assert.equal(caught.mood, "hold");
	assert.match(caught.message, /无法解析/);
});

test("status codes and network failures map to typed Fault moods and messages", async () => {
	const cases = [
		{ status: 404, kind: "join", mood: "warn", pattern: /邀请码/ },
		{ status: 409, kind: "join", mood: "warn", pattern: /占用/ },
		{ status: 403, kind: "repass", mood: "warn", pattern: /当前密码/ },
		{ status: 401, kind: "revive", mood: "warn", pattern: /恢复码/ },
		{ status: 403, kind: "revive", mood: "warn", pattern: /停用/ },
		{ status: 400, kind: "profile", mood: "warn", pattern: /显示名称/ },
		{ status: 429, kind: "login", mood: "hold", pattern: /频繁/ },
		{ status: 503, kind: "read", mood: "hold", pattern: /不可用/ },
		{ status: 0, kind: "read", mood: "hold", pattern: /网络/ },
	];
	for (const item of cases) {
		const info = explain(item.status, item.kind);
		assert.equal(info.mood, item.mood);
		assert.match(info.text, item.pattern);
	}
	const { api } = mock(async () => {
		throw new TypeError("offline");
	});
	let caught;
	try {
		await api.whoami();
	} catch (err) {
		caught = err;
	}
	assert.ok(caught instanceof Fault);
	assert.equal(caught.status, 0);
	assert.equal(caught.code, "NET");
	assert.equal(caught.mood, "hold");
});
