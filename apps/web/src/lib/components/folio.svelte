<script lang="ts">
import { Board, Button, Note, Split, Text } from "@perishlab/design";
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
import Settings from "./settings.svelte";

let {
	present,
}: {
	present?: (next: { system?: string; tone?: Tone }) => void;
} = $props();

let scope: Scope = $state("app");
let tone: Tone = $state("light");
let enabled = $state(false);
let absent = $state(false);
let starting = $state(false);
let fault: string | undefined = $state();
let epoch = 0;
let alive = true;
let again = false;

const store = open({ access: held, device: () => tone });
let snapshot: Document = $state(store.client.state);

let paused = $derived(snapshot.phase === "unknown");
let saving = $derived(snapshot.phase === "ready" && snapshot.pending > 0);
let busy = $derived(snapshot.phase === "loading" || saving || starting);
let current = $derived(views(snapshot.document ?? snapshot.confirmed?.document));
let warning = $derived(fault ?? notice(snapshot));

function sync(): void {
	const confirmed = views(snapshot.confirmed?.document)?.app.presentation;
	const family = store.system.read();
	const mode = store.tone.read();
	const next = {
		system: family.state === "value" ? family.value : confirmed?.system,
		tone:
			mode.state === "value" && (mode.value === "light" || mode.value === "dark")
				? mode.value
				: confirmed?.tone,
	};
	present?.(next);
}

function active(): boolean {
	return alive && enabled;
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
	if (!alive || starting) return;
	if (!reset && blocked()) return;
	starting = true;
	fault = undefined;
	try {
		const config = await configuration();
		if (!alive) return;
		if (!config) {
			absent = true;
			return;
		}
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
			if (alive) location.assign(target);
			return;
		}
		if (reset && store.client.state.phase === "unknown") epoch += 1;
		await store.client.select({
			key: "preferences",
			identity: `${access}:${tone}:${epoch}`,
		});
		sync();
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
	if (!alive || busy) return;
	fault = undefined;
	const access = held();
	if (!access || !enabled) {
		await bind(true);
		return;
	}
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
		sync();
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
	void bind();
	return () => {
		alive = false;
		stop();
		media.removeEventListener("change", change);
	};
});
</script>

<Board title="主题与外观偏好" line="通过 Folio 同步此应用或跨应用共享的视觉设置。">
	{#if absent}
		<Note text="当前实例未启用远程偏好设置服务，你可以直接使用顶部导航栏的外观按钮在浅色与深色模式之间切换。" />
	{:else}
		<Split>
			<Button label="此应用" look={scope === "app" ? "solid" : "quiet"} halt={busy || paused} press={() => { scope = "app"; }} />
			<Button label="共享设置" look={scope === "shared" ? "solid" : "quiet"} halt={busy || paused} press={() => { scope = "shared"; }} />
		</Split>
		<Text>{scope === "app" ? "仅应用于 Ensign 的主题与外观设置。" : "各应用可继承的共享主题与外观设置。"}</Text>
		{#if current}
			<Settings
				view={current[scope]}
				level={scope}
				look="list"
				busy={busy || paused}
				error={warning}
				change={(name, value) => save(name, value)}
				restore={(name) => save(name)}
			/>
		{/if}
		{#if busy}
			<Note text="正在同步设置…" live />
		{/if}
		{#if warning}
			<Note text={warning} mood="warn" live>
				<Button label="重新读取设置" look="quiet" {busy} press={() => void reload()} />
			</Note>
		{/if}
	{/if}
</Board>
