<script lang="ts">
import { Button, Link, Note, Shell, Split, Text } from "@perishlab/design";
import { onMount } from "svelte";
import { configuration, forget, held, Refusal, start } from "../../../authorization.ts";
import { Failure, intent, read, type Scope, type Views, write } from "../../../preferences.ts";
import Identity from "./identity.svelte";
import Settings from "./settings.svelte";

let { leaving = false, settings = false }: { leaving?: boolean; settings?: boolean } = $props();
let views: Views | undefined = $state();
let scope: Scope = $state("app");
let tone: "light" | "dark" = $state("light");
let busy = $state(false);
let enabled = $state(false);
let warning: string | undefined = $state();
let who = false;
let alive = true;
let again = false;

async function load(): Promise<void> {
	if (!who || leaving) return;
	if (busy) { again = true; return; }
	busy = true;
	warning = undefined;
	try {
		const config = await configuration();
		if (!alive || !config) return;
		enabled = true;
		const access = held();
		if (!access) {
			const target = await start(config, { origin: location.origin, back: location.pathname, storage: sessionStorage, crypto });
			if (alive) location.assign(target);
			return;
		}
		const next = await read({ access, tone });
		if (alive) views = next;
	} catch (error) {
		if (error instanceof Failure && error.status === 401) forget();
		if (alive) warning = error instanceof Failure || error instanceof Refusal ? error.message : "暂时无法读取设置，请重试。";
	} finally {
		busy = false;
		if (again && alive) { again = false; void load(); }
	}
}

function identify(present: boolean): void {
	who = present;
	if (!present) { forget(); views = undefined; return; }
	void load();
}

async function save(name: string, value?: string): Promise<void> {
	const access = held();
	if (busy || !views) return;
	if (!access) { forget(); void load(); return; }
	busy = true;
	warning = undefined;
	try {
		const next = await write({ access, tone }, views[scope], intent(scope, name, value));
		if (alive) views = next;
	} catch (error) {
		if (error instanceof Failure && error.status === 401) forget();
		if (alive) warning = error instanceof Failure || error instanceof Refusal ? error.message : "暂时无法完成操作，请重新读取后重试。";
	} finally {
		busy = false;
		if (again && alive) { again = false; void load(); }
	}
}

onMount(() => {
	const media = matchMedia("(prefers-color-scheme: dark)");
	const change = () => { tone = media.matches ? "dark" : "light"; void load(); };
	change();
	media.addEventListener("change", change);
	return () => { alive = false; media.removeEventListener("change", change); };
});
</script>

<Shell system={views?.app.presentation.system ?? "google-accounts"} tone={views?.app.presentation.tone}>
	<Identity {leaving} {settings} ready={identify}>
		{#if enabled || warning}
			{#if settings}
				<Split><Button label="此应用" look={scope === "app" ? "solid" : "quiet"} halt={busy} press={() => scope = "app"} /><Button label="共享设置" look={scope === "shared" ? "solid" : "quiet"} halt={busy} press={() => scope = "shared"} /></Split>
				<Text>{scope === "app" ? "此应用的主题与外观" : "各应用可继承的主题与外观"}</Text>
			{/if}
			{#if views}<Settings state={views[scope]} level={scope} look={settings ? "list" : "line"} {busy} error={warning} change={(name, value) => void save(name, value)} restore={name => void save(name)} />{/if}
			{#if busy}<Note text="正在同步设置…" live />{/if}
			{#if warning}<Note text={warning} mood="warn" live /><Button label="重新读取设置" look="quiet" {busy} press={() => void load()} />{/if}
			<Link href={settings ? "/login" : "/settings"} label={settings ? "返回快捷设置" : "全部设置"} />
		{/if}
	</Identity>
</Shell>
