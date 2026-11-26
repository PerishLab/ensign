import assert from "node:assert/strict";
import test from "node:test";
import { destination, message, submit } from "../protocol.mjs";

const origin = "https://ensign.example";

test("return keeps an authorization request on the issuer origin", () => {
	const target =
		"/api/authorize?client_id=preview&redirect_uri=https%3A%2F%2Fpreview.example%2Fcallback&state=abc";
	assert.equal(destination(target, origin), target);
	assert.equal(
		destination("/x/../api/authorize?state=x", origin),
		"/api/authorize?state=x",
	);
	assert.equal(destination("/", origin), "/");
});

test("untrusted return destinations and authentication loops are refused", () => {
	for (const raw of [
		null,
		"",
		"https://evil.example",
		"javascript:alert(1)",
		"//evil.example",
		"/\\evil.example",
		"/%2f%2fevil.example",
		"/%252f%252fevil.example",
		"/\nevil.example",
		"/login",
		"/login/",
		"/logout",
		"/logout/",
		"/join",
		"/join/",
		"/recover",
		"/recover/",
		"/x/../login",
		"/x/../join///?code=abc",
		"/login?return=/api/authorize",
	]) {
		assert.equal(destination(raw, origin), null, String(raw));
	}
});

test("credentials are submitted only in a same-origin JSON POST", async () => {
	let observed;
	const fetcher = async (...args) => {
		observed = args;
		return { status: 201 };
	};
	await submit(fetcher, "/api/login", {
		login: "ada",
		pass: "private-password",
	});
	assert.equal(observed[0], "/api/login");
	assert.equal(observed[1].method, "POST");
	assert.equal(observed[1].credentials, "same-origin");
	assert.equal(observed[1].redirect, "error");
	assert.equal(observed[1].cache, "no-store");
	assert.ok(observed[1].signal instanceof AbortSignal);
	assert.deepEqual(observed[1].headers, { "Content-Type": "application/json" });
	assert.deepEqual(JSON.parse(observed[1].body), {
		login: "ada",
		pass: "private-password",
	});
});

test("status codes map to safe user-facing messages", () => {
	assert.match(message(401), /账号或密码/);
	assert.match(message(403), /账号或密码/);
	assert.match(message(429), /频繁/);
	assert.match(message(500), /暂时无法完成/);
	assert.match(message(503), /暂时无法完成/);
});
