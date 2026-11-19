import assert from "node:assert/strict";
import { webcrypto } from "node:crypto";
import test from "node:test";
import {
	complete,
	configuration,
	forget,
	held,
	start,
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
