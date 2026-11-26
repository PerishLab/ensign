import assert from "node:assert/strict";
import test from "node:test";
import { folio, runtime } from "../authorization.ts";
import {
	center,
	effective,
	menu,
	open,
	override,
	quick,
} from "../preferences.ts";

function snapshot(system = "material", tone = "dark") {
	const missing = { state: "missing" };
	return {
		revision: "r1",
		document: {
			shared: {
				app: null,
				items: {
					"theme.family": {
						name: "theme.family",
						options: ["google-accounts", "material"],
						shared: { state: "present", value: "google-accounts" },
						app: missing,
						effective: "google-accounts",
						source: "shared",
						editable: true,
					},
				},
			},
			app: {
				app: "ensign",
				presentation: { system, tone, basis: "app" },
				items: {
					"theme.family": {
						name: "theme.family",
						options: ["google-accounts", "material"],
						shared: { state: "present", value: "google-accounts" },
						app: { state: "present", value: system },
						effective: system,
						source: "app",
						editable: true,
					},
					"theme.tone": {
						name: "theme.tone",
						options: ["system", "light", "dark"],
						shared: missing,
						app: { state: "present", value: tone },
						effective: tone,
						source: "app",
						editable: true,
					},
				},
			},
		},
	};
}

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

test("settings quick-switch writes strictly to ensign app scope and reads only app view", () => {
	assert.deepEqual(override("theme.family", "material"), {
		op: "set",
		scope: { kind: "app", name: "ensign" },
		name: "theme.family",
		value: "material",
	});
	assert.deepEqual(override("theme.tone"), {
		op: "reset",
		scope: { kind: "app", name: "ensign" },
		name: "theme.tone",
	});
	const view = quick(snapshot().document);
	assert.equal(view?.app, "ensign");
	assert.deepEqual(Object.keys(view?.items ?? {}), [
		"theme.family",
		"theme.tone",
	]);
	assert.equal(quick(undefined), undefined);
});

test("folio center distinguishes configured, absent, and unknown client configuration states", async () => {
	const config = {
		issuer: "https://identity.example/api",
		client: "ensignweb",
		folio: "https://folio.workshop.liberte.host",
	};
	const discovery = {
		issuer: config.issuer,
		authorization_endpoint: `${config.issuer}/authorize`,
		token_endpoint: `${config.issuer}/token`,
	};
	let calls = 0;
	const loaded = await runtime(async () =>
		Response.json(++calls === 1 ? config : discovery),
	);
	assert.deepEqual(center(loaded.folio), {
		href: "https://folio.workshop.liberte.host/",
		label: "前往 Folio 全域设置中心",
	});
	assert.deepEqual(center(undefined), {});

	const absent = await folio(async () => Response.json({}));
	const calm = center(absent);
	assert.equal(calm.href, undefined);
	assert.equal(calm.mood, "calm");
	assert.match(calm.note ?? "", /未配置 Folio 全域设置中心/);

	const unknown = await folio(async () => new Response(null, { status: 503 }));
	const warn = center(unknown);
	assert.equal(warn.href, undefined);
	assert.equal(warn.mood, "warn");
	assert.match(warn.note ?? "", /暂时无法读取客户端配置/);
});

test("account menu includes Folio center only when runtime folio URL is configured", () => {
	const plain = menu(undefined);
	assert.deepEqual(
		plain.map((item) => item.value),
		["/", "/security", "/sessions", "/settings", "/logout"],
	);
	const linked = menu("https://folio.workshop.liberte.host/");
	assert.deepEqual(
		linked.map((item) => item.value),
		[
			"/",
			"/security",
			"/sessions",
			"/settings",
			"https://folio.workshop.liberte.host/",
			"/logout",
		],
	);
	assert.equal(linked[4].label, "Folio 全域设置中心");
});

test("effective theme handles signed-out, ready, unknown, and failed phases without stale defaults", async () => {
	let fail = false;
	const store = open({
		access: () => "held",
		device: () => "light",
		fetcher: async (_url, init) => {
			if (init?.method === "PATCH" || fail) {
				return Response.json({ error: "down" }, { status: 503 });
			}
			return Response.json(snapshot("material", "dark"));
		},
	});
	assert.equal(effective(store.client.state.document, store), undefined);

	await store.client.select({ key: "preferences", identity: "u1:light:0" });
	assert.deepEqual(effective(store.client.state.document, store), {
		system: "material",
		tone: "dark",
	});

	store.client.update(override("theme.tone", "light"));
	const paused = await until(store.client, (next) => next.phase === "unknown");
	assert.deepEqual(
		effective(paused.document ?? paused.confirmed?.document, store),
		{ system: "material", tone: "dark" },
	);

	fail = true;
	await store.client.select({ key: "preferences", identity: "u1:light:1" });
	const failed = store.client.state;
	assert.equal(failed.phase, "failed");
	assert.equal(
		effective(failed.document ?? failed.confirmed?.document, store),
		undefined,
	);
});
