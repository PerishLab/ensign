import assert from "node:assert/strict";
import test from "node:test";
import { boot, destination, submit } from "../client.mjs";

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
		"/logout",
		"/x/../login",
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
	assert.deepEqual(JSON.parse(observed[1].body), {
		login: "ada",
		pass: "private-password",
	});
});

function rig(path = "/login", search = "") {
	const ids = [
		"login",
		"logout",
		"status",
		"title",
		"intro",
		"identity",
		"session",
		"again",
		"handle",
		"password",
	];
	const elements = Object.fromEntries(
		ids.map((id) => {
			const node = {
				hidden: true,
				textContent: "",
				value: "",
				listeners: {},
				button: { disabled: false },
			};
			Reflect.set(node, "addEventListener", (event, fn) => {
				node.listeners[event] = fn;
			});
			Reflect.set(node, "querySelector", () => node.button);
			return [id, node];
		}),
	);
	const moves = [];
	const location = {
		origin,
		pathname: path,
		search,
		assign(target) {
			moves.push(target);
		},
	};
	const doc = { title: "" };
	Reflect.set(doc, "getElementById", (id) => elements[id]);
	return { elements, location, doc, moves };
}

test("failed login clears the password and leaves a usable form", async () => {
	const r = rig("/login", "?return=%2Fapi%2Fauthorize%3Fstate%3Dx");
	const fetcher = async () => ({ status: 401 });
	await boot(r.doc, r.location, fetcher);
	assert.equal(r.elements.login.hidden, false);
	r.elements.handle.value = "ada";
	r.elements.password.value = "wrong-password";
	await r.elements.login.listeners.submit(
		new Event("submit", { cancelable: true }),
	);
	assert.equal(r.elements.password.value, "");
	assert.equal(r.elements.login.button.disabled, false);
	assert.match(r.elements.status.textContent, /账号或密码/);
	assert.deepEqual(r.moves, []);
});

test("successful login returns to the original authorization request", async () => {
	const r = rig("/login", "?return=%2Fapi%2Fauthorize%3Fstate%3Dx");
	await boot(r.doc, r.location, async (path) => ({
		status: path === "/api/whoami" ? 401 : 201,
	}));
	r.elements.handle.value = "ada";
	r.elements.password.value = "valid-password";
	await r.elements.login.listeners.submit(
		new Event("submit", { cancelable: true }),
	);
	assert.deepEqual(r.moves, ["/api/authorize?state=x"]);
	assert.equal(r.elements.password.value, "");
});

test("an authenticated default page renders identity without a redirect loop", async () => {
	const r = rig();
	await boot(r.doc, r.location, async () => ({
		status: 200,
		json: async () => ({ name: "<b>Ada</b>", login: "ada" }),
	}));
	assert.deepEqual(r.moves, []);
	assert.equal(r.elements.session.hidden, false);
	assert.equal(r.elements.login.hidden, true);
	assert.equal(r.elements.identity.textContent, "<b>Ada</b> (ada)");
});

test("an external return never becomes navigation after successful login", async () => {
	const r = rig("/login", "?return=%2F%2Fevil.example");
	await boot(r.doc, r.location, async (path) => ({
		status: path === "/api/whoami" ? 401 : 201,
	}));
	await r.elements.login.listeners.submit(
		new Event("submit", { cancelable: true }),
	);
	assert.deepEqual(r.moves, ["/login"]);
});

test("logout waits for the real session revocation and shows signed-out state", async () => {
	const r = rig("/logout");
	const calls = [];
	await boot(r.doc, r.location, async (path, options) => {
		calls.push([path, options]);
		return path === "/api/whoami"
			? { status: 200, json: async () => ({ name: "Ada", login: "ada" }) }
			: { status: 204 };
	});
	assert.equal(r.elements.logout.hidden, false);
	assert.equal(calls.length, 1);
	await r.elements.logout.listeners.submit(
		new Event("submit", { cancelable: true }),
	);
	assert.equal(calls[1][0], "/api/logout");
	assert.equal(calls[1][1].method, "POST");
	assert.equal(r.elements.logout.hidden, true);
	assert.equal(r.elements.again.hidden, false);
	assert.match(r.elements.status.textContent, /已退出/);
});

test("a failed logout remains retryable and does not claim revocation", async () => {
	const r = rig("/logout");
	await boot(r.doc, r.location, async (path) =>
		path === "/api/whoami"
			? { status: 200, json: async () => ({ name: "Ada", login: "ada" }) }
			: { status: 503 },
	);
	await r.elements.logout.listeners.submit(
		new Event("submit", { cancelable: true }),
	);
	assert.equal(r.elements.logout.hidden, false);
	assert.equal(r.elements.logout.button.disabled, false);
	assert.doesNotMatch(r.elements.status.textContent, /已退出/);
});

test("session lookup failure leaves a usable login form with a bounded request", async () => {
	const r = rig();
	let observed;
	await boot(r.doc, r.location, async (_path, options) => {
		observed = options;
		throw new Error("offline");
	});
	assert.ok(observed.signal instanceof AbortSignal);
	assert.equal(r.elements.login.hidden, false);
	assert.match(r.elements.status.textContent, /无法连接/);
});

test("an expired session at logout returns the browser to signed-out state", async () => {
	const r = rig("/logout");
	await boot(r.doc, r.location, async (path) =>
		path === "/api/whoami"
			? { status: 200, json: async () => ({ name: "Ada", login: "ada" }) }
			: { status: 401 },
	);
	await r.elements.logout.listeners.submit(
		new Event("submit", { cancelable: true }),
	);
	assert.equal(r.elements.logout.hidden, true);
	assert.equal(r.elements.again.hidden, false);
	assert.match(r.elements.status.textContent, /会话已结束/);
});
