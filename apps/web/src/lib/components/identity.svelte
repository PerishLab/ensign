<script lang="ts">
import {
	Button,
	Field,
	Form,
	Frame,
	Hero,
	Link,
	Note,
	Sheet,
	Split,
	Text,
} from "@perishlab/design";
import { onMount, type Snippet } from "svelte";
import { forget } from "../../../authorization.ts";
import { destination, message, submit } from "../../../protocol.mjs";

let { leaving = false, settings = false, ready, children }: { leaving?: boolean; settings?: boolean; ready?: (present: boolean) => void; children?: Snippet } = $props();
type Actor = { name: string; login: string };
let who: Actor | undefined = $state();
let login = $state("");
let pass = $state("");
let busy = $state(false);
let loading = $state(true);
let warning = $state("");
let ended = $state(false);
let checked = $state(false);
let title = $derived(leaving ? "退出登录" : who ? settings ? "设置" : "你已登录" : "登录");
let line = $derived(
	leaving
		? "退出此设备上的 Ensign 会话。"
		: who
			? "你的身份已准备就绪。"
			: "使用你的 Ensign 账号继续。",
);
const raw = new URLSearchParams(globalThis.location?.search ?? "").get(
	"return",
);
const target = destination(raw, globalThis.location?.origin ?? "");
async function inspect(): Promise<void> {
	loading = true;
	try {
		const response = await fetch("/api/whoami", {
			credentials: "same-origin",
			cache: "no-store",
			redirect: "error",
			signal: AbortSignal.timeout(15000),
		});
		switch (response.status) {
			case 200:
				who = await response.json();
				if (target && !leaving) { globalThis.location.assign(target); return; }
				ready?.(true);
				break;
			case 401:
				ended = leaving;
				ready?.(false);
				break;
			default:
				warning = "暂时无法确认会话状态，你可以重新登录。";
		}
	} catch {
		warning = "无法连接服务，请检查网络后重试。";
	} finally {
		loading = false;
	}
}
async function send(): Promise<void> {
	if (busy) return;
	busy = true;
	warning = "";
	try {
		const response = await submit(fetch, "/api/login", { login, pass });
		if (response.status === 201) globalThis.location.assign(target ?? "/login");
		else warning = message(response.status);
	} catch {
		warning = "无法连接服务，请检查网络后重试。";
	} finally {
		pass = "";
		busy = false;
	}
}
async function leave(): Promise<void> {
	if (busy) return;
	busy = true;
	warning = "";
	try {
		const response = await submit(fetch, "/api/logout", {});
		if (response.status === 204 || response.status === 401) {
			forget();
			who = undefined;
			ended = true;
		} else warning = message(response.status);
	} catch {
		warning = "无法连接服务，请稍后重试退出。";
	} finally {
		busy = false;
	}
}
onMount(() => {
	checked = true;
	void inspect();
});
</script>

<svelte:head><title>Ensign · {title}</title></svelte:head>

<Frame look="center">
	<Sheet look="spread">
		{#snippet lead()}<Hero title={title} {line} mark="ensign" look="brief" />{/snippet}
		{#if loading}<Note text="正在确认会话…" live />
		{:else if ended}<Text>已退出登录。你可以放心关闭此页面。</Text><Link href="/login" label="返回登录" />
		{:else if leaving}<Form label="退出登录" submit={leave} {busy}><Text>{who ? `${who.name} · ${who.login}` : "退出当前会话"}</Text><Text>再次访问受保护内容时，需要重新登录。</Text><Split><Link href="/login" label="取消" /><Button label={busy ? "正在退出…" : "退出登录"} submit {busy} /></Split></Form>
		{:else if who}<Text>{who.name} · {who.login}</Text>{@render children?.()}<Split><Link href="/logout" label="退出登录" />{#if target}<Button label="继续" href={target} />{/if}</Split>
		{:else}<Form label="登录" submit={send} {busy}>
			<Field label="账号" name="login" autocomplete="username" look="float" bind:value={login} required disabled={busy} />
			<Field label="密码" name="pass" autocomplete="current-password" kind="password" look="float" bind:value={pass} required disabled={busy} />
			<Note text="仅在信任的设备上保持登录。" /><Split look="end"><Button label={busy ? "正在登录…" : "登录"} submit {busy} /></Split>
		</Form>{/if}
		{#snippet foot()}{#if warning}<Note text={warning} mood="warn" live />{/if}{#if checked && raw !== null && target === null}<Note text="返回地址无效，登录后将停留在此处。" />{/if}{/snippet}
	</Sheet>
</Frame>
