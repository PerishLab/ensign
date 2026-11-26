<script lang="ts">
import { Board, Button, Grid, Note, Split, Text } from "@perishlab/design";
import { onMount } from "svelte";
import {
	forget,
	held,
	type Lookup,
	Refusal,
	runtime,
	start,
} from "../../../authorization.ts";
import {
	center,
	type Document,
	effective,
	Failure,
	notice,
	open,
	override,
	quick,
	type Tone,
} from "../../../preferences.ts";
import Settings from "./settings.svelte";

let {
	present,
}: {
	present?: (next: { system?: string; tone?: Tone }) => void;
} = $props();

let tone: Tone = $state("light");
let enabled = $state(false);
let absent = $state(false);
let starting = $state(false);
let lookup: Lookup | undefined = $state();
let fault: string | undefined = $state();
let epoch = 0;
let alive = true;
let again = false;

const store = open({ access: held, device: () => tone });
let snapshot: Document = $state(store.client.state);

let paused = $derived(snapshot.phase === "unknown");
let saving = $derived(snapshot.phase === "ready" && snapshot.pending > 0);
let busy = $derived(snapshot.phase === "loading" || saving || starting);
let current = $derived(quick(snapshot.document ?? snapshot.confirmed?.document));
let destination = $derived(center(lookup));
let warning = $derived(fault ?? notice(snapshot));

function sync(): void {
	const next = effective(
		snapshot.document ?? snapshot.confirmed?.document,
		store,
	);
	if (next) present?.(next);
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
		const loaded = await runtime();
		if (!alive) return;
		lookup = loaded.folio;
		if (loaded.fault) {
			fault = loaded.fault.message;
			return;
		}
		if (!loaded.config) {
			absent = true;
			return;
		}
		if (!reset && blocked()) return;
		enabled = true;
		const access = held();
		if (!access) {
			const target = await start(loaded.config, {
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
		if (!alive) return;
		fault =
			error instanceof Refusal ? error.message : "暂时无法读取设置，请重试。";
		if (!lookup) lookup = { status: "unknown" };
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
	store.client.update(override(name, value));
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

<Grid cols={1}>
	<Board title="Ensign 外观快捷切换" line="快速切换当前应用的主题家族与深浅外观，并同步至 Folio。">
		{#if absent}
			<Note text="当前实例未启用远程偏好同步，你可以直接使用顶部导航栏的外观按钮在浅色与深色模式之间切换。" />
		{:else}
			<Text>仅覆盖 Ensign 应用内的主题与外观设置；选择“继承共享或默认设置”将跟随 Folio 共享默认值。</Text>
			{#if current}
				<Settings
					view={current}
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

	<Board title="Folio 全域设置中心" line="共享默认值与跨应用覆写由 Folio 统一管理。">
		<Text>跨应用共享默认值（shared）、各应用专属覆写（ensign、folio 等）及扩展配置均在 Folio 全域设置中心统一管理。</Text>
		{#if destination.href && destination.label}
			<Split look="end">
				<Button label={destination.label} href={destination.href} look="solid" sign="away" />
			</Split>
		{:else if destination.note}
			<Note text={destination.note} mood={destination.mood ?? "calm"} />
		{/if}
	</Board>
</Grid>
