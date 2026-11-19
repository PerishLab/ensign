import assert from "node:assert/strict";
import test from "node:test";
import { Failure, intent, read, write } from "../preferences.ts";

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

test("app and shared views retain authoritative results and transmit only device tone", async () => {
	const calls = [];
	const fetcher = async (url, init) => {
		calls.push({ url, init });
		return Response.json({
			revision: "v",
			presentation: { system: "provided", tone: "light", basis: "policy" },
			items: [{ effective: "dark", source: "policy", editable: false }],
		});
	};
	const result = await read({ access: "held", tone: "dark", fetcher });
	assert.equal(calls.length, 2);
	assert.equal(calls[0].url, "/folio/v1/me/settings?app=ensign&tone=dark");
	assert.equal(calls[1].url, "/folio/v1/me/settings?tone=dark");
	for (const { init } of calls) {
		assert.equal(init.credentials, "omit");
		assert.equal(init.headers.authorization, "Bearer held");
	}
	assert.equal(result.app.presentation.tone, "light");
	assert.equal(result.shared.items[0].source, "policy");
	assert.equal(result.shared.items[0].editable, false);
});

test("conflict never retries a write or manufactures new facts", async () => {
	const before = Object.freeze({ revision: "old", items: Object.freeze([]) });
	let calls = 0;
	const context = {
		access: "held",
		tone: "dark",
		fetcher: async () => {
			calls++;
			return Response.json({ error: "conflict" }, { status: 409 });
		},
	};
	await assert.rejects(
		write(context, before, intent("app", "theme.tone", "light")),
		(error) => error instanceof Failure && error.status === 409,
	);
	assert.equal(calls, 1);
	assert.equal(before.revision, "old");
});

test("successful write re-reads both views instead of inferring the effective theme", async () => {
	const calls = [];
	const fetcher = async (url, init) => {
		calls.push({ url, init });
		return Response.json({ revision: "new", presentation: { tone: "light" } });
	};
	const change = intent("shared", "theme.tone", "dark");
	const result = await write(
		{ access: "held", tone: "dark", fetcher },
		{ revision: "old" },
		change,
	);
	assert.deepEqual(JSON.parse(calls[0].init.body), {
		revision: "old",
		changes: [change],
	});
	assert.equal(calls[0].init.method, "PATCH");
	assert.equal(calls.length, 3);
	assert.equal(result.app.presentation.tone, "light");
});
