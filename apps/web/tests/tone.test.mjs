import assert from "node:assert/strict";
import test from "node:test";
import { forget, shown, wear } from "../authorization.ts";
import { read, save, stored, watch } from "../tone.ts";

function query(initial) {
	return Object.assign(new EventTarget(), { matches: initial });
}

test("tone follows OS prefers-color-scheme until explicitly pinned and stops on unsubscribe", () => {
	sessionStorage.clear();
	const media = query(false);
	const seen = [];
	assert.equal(stored(sessionStorage), undefined);
	assert.equal(read(sessionStorage, media), "light");

	const stop = watch((next) => seen.push(next), media, sessionStorage);
	media.matches = true;
	media.dispatchEvent(new Event("change"));
	assert.deepEqual(seen, ["dark"]);
	assert.equal(read(sessionStorage, media), "dark");

	save("light", sessionStorage);
	assert.equal(stored(sessionStorage), "light");
	assert.equal(read(sessionStorage, media), "light");

	media.matches = false;
	media.dispatchEvent(new Event("change"));
	media.matches = true;
	media.dispatchEvent(new Event("change"));
	assert.deepEqual(seen, ["dark"]);

	sessionStorage.clear();
	stop();
	media.matches = false;
	media.dispatchEvent(new Event("change"));
	assert.deepEqual(seen, ["dark"]);
});

test("Folio in-memory presentation never writes to local shade storage or blocks OS reactivity", () => {
	sessionStorage.clear();
	forget();
	const media = query(false);
	const seen = [];
	const stop = watch((next) => seen.push(next), media, sessionStorage);

	wear({ system: "material", tone: "dark" });
	assert.deepEqual(shown(), { system: "material", tone: "dark" });
	assert.equal(stored(sessionStorage), undefined);
	assert.equal(sessionStorage.length, 0);

	media.matches = true;
	media.dispatchEvent(new Event("change"));
	assert.deepEqual(seen, ["dark"]);

	forget();
	assert.equal(shown(), undefined);
	stop();
});
