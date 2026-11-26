import assert from "node:assert/strict";
import test from "node:test";
import { Fault } from "../account.ts";
import {
	Admin,
	describe,
	validapp,
	validnote,
	validorigin,
	validredirect,
	validslug,
} from "../admin.ts";

function mock(handler) {
	const calls = [];
	const fetcher = async function (url, init) {
		assert.equal(this, undefined);
		calls.push({ url, init });
		return handler(url, init);
	};
	return { api: new Admin(fetcher), calls };
}

test("validators enforce invite notes, slugs, origins, redirects and mode rules", () => {
	assert.equal(validnote(""), true);
	assert.equal(validnote("a".repeat(120)), true);
	assert.equal(validnote("a".repeat(121)), false);
	assert.equal(validnote("bad\nnote"), false);

	assert.equal(validslug("a"), true);
	assert.equal(validslug("codehull-web-1"), true);
	assert.equal(validslug("-bad"), false);
	assert.equal(validslug("bad-"), false);
	assert.equal(validslug("BadSlug"), false);
	assert.equal(validslug("bad_slug"), false);
	assert.equal(validslug("a".repeat(65)), false);

	assert.equal(validorigin("https://app.example"), true);
	assert.equal(validorigin("http://localhost:5173"), true);
	assert.equal(validorigin("http://127.0.0.1:8080/home"), true);
	assert.equal(validorigin("http://app.example"), false);
	assert.equal(validorigin("https://app.example#frag"), false);
	assert.equal(validorigin("https://user@app.example"), false);
	assert.equal(validorigin("https://@app.example"), false);
	assert.equal(validorigin("HTTPS://app.example"), false);
	assert.equal(validorigin("HTTP://localhost:5173"), false);
	assert.equal(validorigin("https://app.example\\path"), false);
	assert.equal(validorigin("https://app.example/\u00a0space"), false);

	assert.equal(validredirect("https://app.example/callback"), true);
	assert.equal(validredirect("https://app.example/callback?a=1"), false);

	assert.match(validapp("oidc", "", "https://a.example/cb") ?? "", /主页/);
	assert.match(validapp("oidc", "https://a.example", "") ?? "", /回调/);
	assert.equal(
		validapp("oidc", "https://a.example", "https://a.example/cb"),
		undefined,
	);
	assert.equal(validapp("forward", "https://a.example", ""), undefined);
	assert.match(validapp("forward", "", "") ?? "", /主页/);
	assert.equal(validapp("resource", "", ""), undefined);
});

test("all 13 admin endpoints use exact paths, methods, bodies and fetch guards", async () => {
	const actor = {
		id: 2,
		sub: "sub-2",
		login: "ada",
		name: "Ada",
		kind: "user",
		barred: false,
		created: 1700000000,
		teams: ["Ops"],
	};
	const invite = {
		id: 7,
		note: "ops",
		created: 1700000000,
		expires: 1700086400,
	};
	const team = { id: 3, name: "Ops", created: 1700000000, members: [] };
	const app = {
		id: 9,
		name: "Codehull",
		slug: "codehull",
		home: "https://code.example",
		redirect: "https://code.example/cb",
		mode: "oidc",
		barred: false,
		created: 1700000000,
	};
	const replies = {
		"GET /api/admin/actors": () => Response.json([actor]),
		"GET /api/admin/actors/2": () => Response.json(actor),
		"PATCH /api/admin/actors/2": () =>
			Response.json({ ...actor, barred: true }),
		"GET /api/admin/invites": () => Response.json([invite]),
		"POST /api/admin/invites": () =>
			Response.json({ ...invite, code: "secret-code" }, { status: 201 }),
		"DELETE /api/admin/invites/7": () => new Response(null, { status: 204 }),
		"GET /api/admin/teams": () => Response.json([team]),
		"POST /api/admin/teams": () => Response.json(team, { status: 201 }),
		"POST /api/admin/teams/3/members": () =>
			Response.json({ tie: 11, team: 3, actor: 2 }, { status: 201 }),
		"DELETE /api/admin/teams/3/members/11": () =>
			new Response(null, { status: 204 }),
		"GET /api/admin/apps": () => Response.json([app]),
		"POST /api/admin/apps": () => Response.json(app, { status: 201 }),
		"PATCH /api/admin/apps/9": () => Response.json({ ...app, barred: true }),
	};
	const { api, calls } = mock((url, init) => {
		const key = `${init.method} ${url}`;
		assert.ok(key in replies, `unexpected call: ${key}`);
		return replies[key]();
	});

	assert.equal((await api.actors()).length, 1);
	assert.equal((await api.actor(2)).sub, "sub-2");
	assert.equal((await api.bar(2, true)).barred, true);
	assert.equal((await api.invites()).length, 1);
	assert.equal(
		(await api.invite({ note: "ops", ttl: 86400 })).code,
		"secret-code",
	);
	await api.cancel(7);
	assert.equal((await api.teams()).length, 1);
	assert.equal((await api.team("Ops")).name, "Ops");
	assert.equal((await api.enroll(3, 2)).tie, 11);
	await api.eject(3, 11);
	assert.equal((await api.apps()).length, 1);
	assert.equal(
		(
			await api.create({
				name: "Codehull",
				slug: "codehull",
				mode: "oidc",
				home: "https://code.example",
				redirect: "https://code.example/cb",
			})
		).slug,
		"codehull",
	);
	assert.equal(
		(
			await api.update(9, {
				name: "Codehull 2",
				barred: true,
				slug: "ignored",
				mode: "resource",
				secret: "never",
			})
		).barred,
		true,
	);

	assert.equal(calls.length, 13);
	for (const entry of calls) {
		assert.equal(entry.init.credentials, "same-origin");
		assert.equal(entry.init.cache, "no-store");
		assert.equal(entry.init.redirect, "error");
		assert.ok(entry.init.signal instanceof AbortSignal);
	}
	assert.deepEqual(JSON.parse(calls[2].init.body), { barred: true });
	assert.deepEqual(JSON.parse(calls[4].init.body), { note: "ops", ttl: 86400 });
	assert.deepEqual(JSON.parse(calls[7].init.body), { name: "Ops" });
	assert.deepEqual(JSON.parse(calls[8].init.body), { actor: 2 });
	assert.deepEqual(JSON.parse(calls[12].init.body), {
		name: "Codehull 2",
		barred: true,
	});
});

function assertfault(err, expect) {
	assert.ok(err instanceof Fault);
	assert.equal(err.status, expect.status);
	if (expect.code !== undefined) assert.equal(err.code, expect.code);
	assert.equal(err.mood, expect.mood);
	assert.match(err.message, expect.pattern);
	return true;
}

test("error epistemology maps 400, 401, 403, 404, 409, 429, 503 and NET accurately", async () => {
	const cases = [
		{ status: 400, action: "bar", mood: "warn", pattern: /自身/ },
		{ status: 401, action: "read", mood: "warn", pattern: /重新登录/ },
		{ status: 403, action: "invite", mood: "warn", pattern: /Keel/ },
		{ status: 404, action: "cancel", mood: "warn", pattern: /不存在/ },
		{ status: 409, action: "bar", mood: "hold", pattern: /ensign/ },
		{ status: 409, action: "create", mood: "hold", pattern: /slug/ },
		{ status: 429, action: "read", mood: "hold", pattern: /频繁/ },
		{ status: 503, action: "read", mood: "hold", pattern: /不可用/ },
		{ status: 503, action: "bar", mood: "hold", pattern: /尚不明确/ },
		{ status: 0, action: "read", mood: "hold", pattern: /网络/ },
		{ status: 0, action: "create", mood: "hold", pattern: /尚不明确/ },
	];
	for (const item of cases) {
		const info = describe(item.status, item.action);
		assert.equal(info.mood, item.mood);
		assert.match(info.text, item.pattern);
	}

	const denied = mock(() => new Response(null, { status: 403 }));
	await assert.rejects(
		() => denied.api.invite({ note: "x" }),
		(err) =>
			assertfault(err, {
				status: 403,
				code: "403",
				mood: "warn",
				pattern: /Keel/,
			}),
	);

	const down = mock(() => new Response(null, { status: 503 }));
	await assert.rejects(
		() => down.api.bar(2, true),
		(err) =>
			assertfault(err, {
				status: 503,
				code: "503",
				mood: "hold",
				pattern: /尚不明确/,
			}),
	);

	const offline = mock(async () => {
		throw new TypeError("offline");
	});
	await assert.rejects(
		() => offline.api.team("Ops"),
		(err) =>
			assertfault(err, {
				status: 0,
				code: "NET",
				mood: "hold",
				pattern: /尚不明确/,
			}),
	);
});

test("malformed JSON on 200 or 201 throws a hold Fault instead of SyntaxError", async () => {
	for (const status of [200, 201]) {
		const { api } = mock(
			() =>
				new Response("not-json", {
					status,
					headers: { "content-type": "text/plain" },
				}),
		);
		const call = status === 200 ? () => api.actors() : () => api.team("Ops");
		await assert.rejects(call, (err) =>
			assertfault(err, { status, mood: "hold", pattern: /无法解析/ }),
		);
	}
});
