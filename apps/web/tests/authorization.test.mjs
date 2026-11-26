import assert from "node:assert/strict";
import { webcrypto } from "node:crypto";
import test from "node:test";
import {
	complete,
	configuration,
	folio,
	forget,
	held,
	portal,
	runtime,
	shown,
	start,
	wear,
} from "../authorization.ts";

const config = { issuer: "https://identity.example/api", client: "ensignweb" };
const origin = "https://web.example";

function vault() {
	sessionStorage.clear();
	return sessionStorage;
}

async function journey(back = "/settings") {
	const storage = vault();
	const url = new URL(
		await start(config, { origin, back, storage, crypto: webcrypto }),
	);
	return {
		storage,
		url,
		callback: `${origin}/callback?code=once&state=${url.searchParams.get("state")}`,
	};
}

test("unconfigured settings leave identity alone; discovery must bind the configured issuer", async () => {
	assert.equal(await configuration(async () => Response.json({})), undefined);
	let calls = 0;
	await assert.rejects(
		configuration(async () =>
			Response.json(
				++calls === 1 ? config : { issuer: "https://foreign.example" },
			),
		),
		/身份/,
	);
	assert.equal(calls, 2);
});

test("S256 binds the stored verifier and exact client, callback, audience and local return", async () => {
	const { storage, url } = await journey();
	const transaction = JSON.parse(storage.getItem("ensign.authorization"));
	const digest = await webcrypto.subtle.digest(
		"SHA-256",
		new TextEncoder().encode(transaction.verifier),
	);
	assert.equal(
		url.searchParams.get("code_challenge"),
		Buffer.from(digest).toString("base64url"),
	);
	assert.equal(url.searchParams.get("code_challenge_method"), "S256");
	assert.equal(url.searchParams.get("redirect_uri"), `${origin}/callback`);
	assert.equal(url.searchParams.get("resource"), "folio");
	assert.equal(url.searchParams.get("client_id"), config.client);
	assert.equal(transaction.back, "/settings");
	assert.equal("access_token" in transaction, false);
	await journey("/security");
	await journey("/sessions");
	await journey("/admin/actors");
	await journey("/admin/invites");
	await journey("/admin/teams");
	await journey("/admin/apps");
	await assert.rejects(journey("//foreign.example"), /返回地址/);
});

test("wrong, missing or duplicate state and changed origin/client consume the transaction without exchange", async () => {
	for (const alter of [
		(url) => url.replace(/state=.*/, "state=wrong"),
		(url) => `${url}&state=duplicate`,
		(url) => url.replace(/&state=.*/, ""),
		(url) => url.replace(origin, "https://foreign.example"),
	]) {
		const { storage, callback } = await journey();
		let calls = 0;
		const seat = {
			url: alter(callback),
			storage,
			fetcher: async () => {
				calls++;
				throw Error("unexpected exchange");
			},
		};
		await assert.rejects(complete(config, seat), /不匹配/);
		assert.equal(calls, 0);
		assert.equal(storage.length, 0);
		await assert.rejects(complete(config, seat), /失效/);
	}
	const { storage, callback } = await journey();
	await assert.rejects(
		complete(
			{ ...config, client: "changed" },
			{ url: callback, storage, fetcher: fetch },
		),
		/不匹配/,
	);
});

test("successful exchange keeps only the access token in memory and cannot replay", async () => {
	const { storage, callback } = await journey();
	let request;
	const fetcher = async (url, init) => {
		request = { url, init };
		return Response.json({
			access_token: "opaque",
			token_type: "Bearer",
			expires_in: 60,
			refresh_token: "not-retained",
		});
	};
	assert.equal(
		await complete(config, { url: callback, storage, fetcher }),
		"/settings",
	);
	assert.equal(request.url, "/api/token");
	assert.equal(request.init.credentials, "omit");
	assert.equal(request.init.redirect, "error");
	assert.equal(request.init.body.get("grant_type"), "authorization_code");
	assert.equal(request.init.body.get("redirect_uri"), `${origin}/callback`);
	assert.equal(held(), "opaque");
	assert.equal(storage.length, 0);
	await assert.rejects(
		complete(config, { url: callback, storage, fetcher }),
		/失效/,
	);
	assert.equal(held(), undefined);
	forget();
});

test("portal accepts clean http/https URLs and ignores missing, empty, or unsafe values", () => {
	assert.equal(
		portal("https://folio.workshop.liberte.host"),
		"https://folio.workshop.liberte.host/",
	);
	assert.equal(
		portal("  https://folio.workshop.liberte.host/settings?tab=theme  "),
		"https://folio.workshop.liberte.host/settings?tab=theme",
	);
	assert.equal(portal("http://localhost:4173"), "http://localhost:4173/");
	for (const raw of [
		undefined,
		null,
		123,
		{},
		"",
		"   ",
		"/settings",
		"//folio.example",
		"folio.example",
		"javascript:alert(1)",
		"data:text/html,hi",
		"ftp://folio.example",
		"https://user:pass@folio.example",
		"https://folio.example/bad path",
		"https://folio.example/\nbad",
	]) {
		assert.equal(portal(raw), undefined, String(raw));
	}
});

test("configuration parses optional folio URL and degrades gracefully when omitted or invalid", async () => {
	const discovery = {
		issuer: config.issuer,
		authorization_endpoint: `${config.issuer}/authorize`,
		token_endpoint: `${config.issuer}/token`,
	};
	const pair = (client) => {
		let calls = 0;
		return async () => Response.json(++calls === 1 ? client : discovery);
	};
	assert.deepEqual(
		await configuration(
			pair({ ...config, folio: "https://folio.workshop.liberte.host" }),
		),
		{ ...config, folio: "https://folio.workshop.liberte.host/" },
	);
	assert.deepEqual(await configuration(pair(config)), config);
	for (const bad of ["", "   ", "javascript:alert(1)", "not-a-url", 42]) {
		assert.deepEqual(
			await configuration(pair({ ...config, folio: bad })),
			config,
		);
	}
	assert.equal(
		await configuration(async () =>
			Response.json({ folio: "https://folio.workshop.liberte.host" }),
		),
		undefined,
	);
});

test("folio and runtime distinguish configured, absent, and unknown without duplicate client fetches", async () => {
	assert.deepEqual(
		await folio(async () =>
			Response.json({ folio: "https://folio.workshop.liberte.host" }),
		),
		{ status: "configured", href: "https://folio.workshop.liberte.host/" },
	);
	assert.deepEqual(await folio(async () => Response.json({})), {
		status: "absent",
	});
	assert.deepEqual(
		await folio(async () => Response.json({ folio: "javascript:alert(1)" })),
		{ status: "absent" },
	);
	assert.deepEqual(
		await folio(async () => new Response(null, { status: 404 })),
		{ status: "absent" },
	);
	for (const reply of [
		async () => new Response(null, { status: 503 }),
		async () => new Response("not-json", { status: 200 }),
		async () => Response.json([]),
		async () => {
			throw new TypeError("offline");
		},
	]) {
		assert.deepEqual(await folio(reply), { status: "unknown" });
	}

	let calls = 0;
	const standalone = await runtime(async () => {
		calls += 1;
		return Response.json({ folio: "https://folio.workshop.liberte.host" });
	});
	assert.equal(calls, 1);
	assert.equal(standalone.config, undefined);
	assert.equal(standalone.fault, undefined);
	assert.deepEqual(standalone.folio, {
		status: "configured",
		href: "https://folio.workshop.liberte.host/",
	});

	let probe = 0;
	const broken = await runtime(async () => {
		probe += 1;
		return probe === 1
			? Response.json({
					...config,
					folio: "https://folio.workshop.liberte.host",
				})
			: new Response(null, { status: 503 });
	});
	assert.equal(probe, 2);
	assert.equal(broken.config, undefined);
	assert.ok(broken.fault);
	assert.deepEqual(broken.folio, {
		status: "configured",
		href: "https://folio.workshop.liberte.host/",
	});

	const offline = await runtime(
		async () => new Response(null, { status: 503 }),
	);
	assert.equal(offline.config, undefined);
	assert.ok(offline.fault);
	assert.deepEqual(offline.folio, { status: "unknown" });
});

test("in-memory presentation clears on forget so logout and 401 prevent cross-user leakage", () => {
	forget();
	assert.equal(shown(), undefined);
	wear({ system: "material", tone: "dark" });
	assert.deepEqual(shown(), { system: "material", tone: "dark" });
	assert.equal(sessionStorage.length, 0);
	forget();
	assert.equal(shown(), undefined);
});
