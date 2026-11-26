import assert from "node:assert/strict";
import { webcrypto } from "node:crypto";
import test from "node:test";
import { gunzipSync } from "node:zlib";
import {
	activate,
	boot,
	endpoint,
	flush,
	locus,
	reset,
	route,
} from "@perishlab/ensign-web/observe";
import { Account, Fault } from "../account.ts";
import { Admin } from "../admin.ts";
import { complete, folio, forget, runtime, start } from "../authorization.ts";

async function decode(body, headers) {
	const raw =
		body instanceof Uint8Array
			? Buffer.from(body)
			: Buffer.from(await new Response(body).arrayBuffer());
	const encoding =
		headers?.["Content-Encoding"] ?? headers?.["content-encoding"] ?? "";
	const bytes = encoding === "gzip" ? gunzipSync(raw) : raw;
	return bytes.toString("utf8");
}

function collector() {
	const sent = [];
	const fetcher = async (url, init) => {
		const text = await decode(init?.body, init?.headers);
		const lines = text
			.trim()
			.split("\n")
			.filter(Boolean)
			.map((line) => JSON.parse(line));
		sent.push({ url: String(url), method: init?.method, lines, raw: text });
		return new Response(null, { status: 204 });
	};
	return { sent, fetcher };
}

test("observe endpoint validator accepts same-origin paths and rejects unsafe values", () => {
	assert.equal(endpoint("/api/observe"), "/api/observe");
	assert.equal(endpoint("  /api/observe  "), "/api/observe");
	for (const bad of [
		undefined,
		null,
		42,
		"",
		"   ",
		"https://evil.example/observe",
		"//evil.example/observe",
		"api/observe",
		"/api/observe bad",
		"/api\\observe",
	]) {
		assert.equal(endpoint(bad), undefined, String(bad));
	}
	assert.equal(route("/api/sessions/42?token=secret"), "/api/sessions/:id");
	assert.equal(
		route("/api/admin/teams/3/members/11#frag"),
		"/api/admin/teams/:id/members/:id",
	);
});

test("boot stays silent no-op when observe is absent and activates when present", async () => {
	await reset();
	assert.equal(await boot(async () => Response.json({})), false);
	assert.equal(
		await boot(async () => Response.json({ observe: "https://evil.example" })),
		false,
	);
	assert.equal(
		await boot(async () => new Response(null, { status: 404 })),
		false,
	);
	assert.equal(
		await boot(async () => Response.json({ observe: "/api/observe" })),
		true,
	);
	await reset();
});

test("observe and authorization share one /client.json fetch and retry transient faults", async () => {
	await reset();
	const issuer = "https://ensign.example/api";
	let calls = 0;
	let discovered = 0;
	const routes = {
		"/client.json": () => {
			calls += 1;
			return Response.json({
				issuer,
				client: "ensignweb",
				folio: "https://folio.example",
				observe: "/api/observe",
			});
		},
		"/api/.well-known/openid-configuration": () => {
			discovered += 1;
			return Response.json({
				issuer,
				authorization_endpoint: `${issuer}/authorize`,
				token_endpoint: `${issuer}/token`,
			});
		},
	};
	const shared = async (url) =>
		routes[url]?.() ?? new Response(null, { status: 404 });

	const [active, portal, loaded] = await Promise.all([
		boot(shared),
		folio(shared),
		runtime(shared),
	]);
	assert.equal(active, true);
	assert.deepEqual(portal, {
		status: "configured",
		href: "https://folio.example/",
	});
	assert.deepEqual(loaded.config, {
		issuer,
		client: "ensignweb",
		folio: "https://folio.example/",
	});
	assert.equal(calls, 1);
	assert.equal(discovered, 1);
	assert.equal(await boot(shared), true);
	assert.equal(calls, 1);

	let attempts = 0;
	const flaky = async () => {
		attempts += 1;
		return attempts === 1
			? new Response(null, { status: 503 })
			: Response.json({ observe: "/api/observe" });
	};
	const [first, second] = await Promise.all([boot(flaky), folio(flaky)]);
	assert.equal(first, false);
	assert.deepEqual(second, { status: "unknown" });
	assert.equal(attempts, 1);
	assert.equal(await boot(flaky), true);
	assert.equal(attempts, 2);
	await reset();
});

test("account, admin and oidc flows emit structured Locus atoms without leaking secrets", async () => {
	await reset();
	const { sent, fetcher } = collector();
	assert.equal(activate("/api/observe", fetcher), true);

	const pass = "hunter2-super-secret-pass";
	const oldpass = "previous-secret-password";
	const rescuecode = "RESCUE-CODE-9999-SECRET";
	const invitecode = "INVITE-CODE-SECRET-8888";
	const oidccode = "OAUTH-CODE-7777-SECRET";
	const accesstoken = "BEARER-ACCESS-TOKEN-SECRET-6666";

	const replies = {
		"/api/login": () => Response.json({ id: 1 }, { status: 201 }),
		"/api/revive": () => new Response(null, { status: 204 }),
		"/api/repass": () => new Response(null, { status: 403 }),
		"/api/whoami": () => new Response(null, { status: 503 }),
		"/api/logout": () => new Response(null, { status: 204 }),
	};
	const account = new Account(
		async (url) => replies[url]?.() ?? new Response(null, { status: 204 }),
	);

	await account.login({ login: "ada", pass });
	await account.revive({ login: "ada", code: rescuecode, pass });
	await assert.rejects(
		() => account.repass({ old: oldpass, pass }),
		(err) => err instanceof Fault && err.status === 403,
	);
	await assert.rejects(
		() => account.whoami(),
		(err) => err instanceof Fault && err.status === 503,
	);
	await account.logout();

	const admin = new Admin(async (url) => {
		if (url === "/api/admin/invites") {
			return Response.json(
				{
					id: 7,
					note: "ops",
					created: 1700000000,
					expires: null,
					code: invitecode,
				},
				{ status: 201 },
			);
		}
		throw new TypeError("network down");
	});
	await admin.invite({ note: "ops" });
	await assert.rejects(() => admin.actors(), Fault);

	sessionStorage.clear();
	const cfg = { issuer: "https://ensign.example/api", client: "ensignweb" };
	const authurl = new URL(
		await start(cfg, {
			origin: "https://ensign.example",
			back: "/settings",
			storage: sessionStorage,
			crypto: webcrypto,
		}),
	);
	const saved = JSON.parse(sessionStorage.getItem("ensign.authorization"));
	const callback = `https://ensign.example/callback?code=${oidccode}&state=${authurl.searchParams.get("state")}`;
	await complete(cfg, {
		url: callback,
		storage: sessionStorage,
		fetcher: async () =>
			Response.json({
				access_token: accesstoken,
				token_type: "Bearer",
				expires_in: 60,
			}),
	});
	forget();

	await flush();
	assert.ok(sent.length > 0);
	assert.ok(sent.every((batch) => batch.url === "/api/observe"));

	const atoms = sent.flatMap((batch) => batch.lines);
	assert.ok(atoms.length > 0);
	assert.ok(atoms.every((atom) => atom.producer === "ensign-web"));
	assert.ok(
		atoms.some((atom) => atom.context["locus.schema"] === "locus.span"),
	);
	assert.ok(
		atoms.some((atom) => atom.context["locus.schema"] === "locus.metric"),
	);
	assert.ok(
		atoms.some((atom) => atom.context["locus.schema"] === "locus.fault"),
	);
	assert.ok(
		atoms.some(
			(atom) =>
				atom.context["locus.schema"] === "locus.log" &&
				atom.context.level === "warn",
		),
	);

	const dump = sent.map((batch) => batch.raw).join("\n");
	for (const secret of [
		pass,
		oldpass,
		rescuecode,
		invitecode,
		oidccode,
		accesstoken,
		saved.verifier,
		saved.state,
	]) {
		assert.equal(dump.includes(secret), false, `leaked ${secret}`);
	}

	await reset();
	locus().info("ignored-after-reset");
	await flush();
});
