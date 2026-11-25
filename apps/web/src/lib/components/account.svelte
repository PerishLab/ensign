<script lang="ts">
import { Button, Link, Note, Shell, Split, Text } from "@perishlab/design";
import { onMount } from "svelte";
import {
	configuration,
	forget,
	held,
	Refusal,
	start,
} from "../../../authorization.ts";
import {
	type Document,
	Failure,
	intent,
	notice,
	open,
	type Scope,
	type Tone,
	views,
} from "../../../preferences.ts";
import Identity from "./identity.svelte";
import Settings from "./settings.svelte";

let {
	leaving = false,
	settings = false,
}: { leaving?: boolean; settings?: boolean } = $props();
let scope: Scope = $state("app");
let tone: Tone = $state("light");
let enabled = $state(false);
let starting = $state(false);
let fault: string | undefined = $state();
let epoch = 0;
let who = false;
let alive = true;
let again = false;

const store = open({ access: held, device: () => tone });
let snapshot: Document = $state(store.client.state);

function theme(
	state: Document,
	fallback: Tone,
): { system: string; tone: Tone } {
	const confirmed = views(state.confirmed?.document)?.app.presentation;
	const family = store.system.read();
	const mode = store.tone.read();
	return {
		system:
			family.state === "value"
				? family.value
				: (confirmed?.system ?? "google-accounts"),
		tone:
			mode.state === "value" &&
			(mode.value === "light" || mode.value === "dark")
				? mode.value
				: (confirmed?.tone ?? fallback),
	};
}

let paused = $derived(snapshot.phase === "unknown");
let saving = $derived(snapshot.phase === "ready" && snapshot.pending > 0);
let busy = $derived(snapshot.phase === "loading" || saving || starting);
let current = $derived(views(snapshot.document ?? snapshot.confirmed?.document));
let warning = $derived(fault ?? notice(snapshot));
let design = $derived(theme(snapshot, tone));

function usable(): boolean {
	return alive && who && !leaving;
}

function active(): boolean {
	return usable() && enabled;
}

function blocked(): boolean {
	const state = store.client.state;
	return state.phase === "unknown" || state.pending > 0;
}

function flush(): void {
	if (!again || !active() || starting) return;
	if (blocked()) return;
	again = false;
	void bind();
}

async function bind(reset = false): Promise<void> {
	if (!usable() || starting) return;
	if (!reset && blocked()) return;
	starting = true;
	fault = undefined;
	try {
		const config = await configuration();
		if (!usable() || !config) return;
		if (!reset && blocked()) return;
		enabled = true;
		const access = held();
		if (!access) {
			const target = await start(config, {
				origin: location.origin,
				back: location.pathname,
				storage: sessionStorage,
				crypto,
			});
			if (usable()) location.assign(target);
			return;
		}
		if (reset && store.client.state.phase === "unknown") epoch += 1;
		await store.client.select({
			key: "preferences",
			identity: `${access}:${tone}:${epoch}`,
		});
	} catch (error) {
		if (alive) {
			fault =
				error instanceof Refusal ? error.message : "暂时无法读取设置，请重试。";
		}
	} finally {
		starting = false;
		flush();
	}
}

async function reload(): Promise<void> {
	if (!usable() || busy) return;
	fault = undefined;
	const access = held();
	if (!access || !enabled) {
		await bind(true);
		return;
	}
	if (!usable()) return;
	if (store.client.state.phase === "unknown") {
		epoch += 1;
		await store.client.select({
			key: "preferences",
			identity: `${access}:${tone}:${epoch}`,
		});
		return;
	}
	await store.client.refresh();
}

function identify(present: boolean): void {
	who = present;
	if (!present) {
		again = false;
		forget();
		fault = undefined;
		void store.client.select(null);
		snapshot = store.client.state;
		return;
	}
	void bind();
}

function save(name: string, value?: string): void {
	if (busy || paused || snapshot.phase !== "ready") return;
	if (!held()) {
		forget();
		void bind();
		return;
	}
	fault = undefined;
	store.client.update(intent(scope, name, value));
	snapshot = store.client.state;
}

onMount(() => {
	const stop = store.client.subscribe(() => {
		if (!alive) return;
		snapshot = store.client.state;
		if (snapshot.phase === "unknown") again = false;
		if (snapshot.error instanceof Failure && snapshot.error.status === 401) {
			forget();
		}
		if (snapshot.phase === "ready" && snapshot.pending === 0) flush();
	});
	const media = matchMedia("(prefers-color-scheme: dark)");
	tone = media.matches ? "dark" : "light";
	const change = () => {
		tone = media.matches ? "dark" : "light";
		if (!active() || store.client.state.phase === "unknown") return;
		if (starting || store.client.state.pending > 0) {
			again = true;
			return;
		}
		void bind();
	};
	media.addEventListener("change", change);
	snapshot = store.client.state;
	return () => {
		alive = false;
		stop();
		media.removeEventListener("change", change);
	};
});
</script>

<Shell system={design.system} tone={design.tone}>
	<Identity {leaving} {settings} ready={identify}>
		{#if enabled || warning}
			{#if settings}
				<Split><Button label="此应用" look={scope === "app" ? "solid" : "quiet"} halt={busy || paused} press={() => { scope = "app"; }} /><Button label="共享设置" look={scope === "shared" ? "solid" : "quiet"} halt={busy || paused} press={() => { scope = "shared"; }} /></Split>
				<Text>{scope === "app" ? "此应用的主题与外观" : "各应用可继承的主题与外观"}</Text>
			{/if}
			{#if current}<Settings view={current[scope]} level={scope} look={settings ? "list" : "line"} busy={busy || paused} error={warning} change={(name, value) => save(name, value)} restore={name => save(name)} />{/if}
			{#if busy}<Note text="正在同步设置…" live />{/if}
			{#if warning}<Note text={warning} mood="warn" live /><Button label="重新读取设置" look="quiet" {busy} press={() => void reload()} />{/if}
			<Link href={settings ? "/login" : "/settings"} label={settings ? "返回快捷设置" : "全部设置"} />
		{/if}
	</Identity>
</Shell>
