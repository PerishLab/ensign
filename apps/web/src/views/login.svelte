<script lang="ts">
import { Button, Field, Form, Link, Note, Split } from "@perishlab/design";
import { onMount } from "svelte";
import { account, Fault } from "../../account.ts";
import { destination } from "../../protocol.mjs";
import Rite from "../lib/components/rite.svelte";

let login = $state("");
let pass = $state("");
let busy = $state(false);
let loading = $state(true);
let fault: Fault | undefined = $state();
let checked = $state(false);
let alive = true;

const raw = new URLSearchParams(globalThis.location?.search ?? "").get(
	"return",
);
const target = destination(raw, globalThis.location?.origin ?? "");

async function inspect(): Promise<void> {
	loading = true;
	fault = undefined;
	let moved = false;
	try {
		await account.whoami();
		if (!alive) return;
		moved = true;
		globalThis.location.assign(target ?? "/");
	} catch (error) {
		if (!alive) return;
		if (error instanceof Fault && error.status === 401) return;
		fault = error instanceof Fault ? error : new Fault(0, "read");
	} finally {
		if (alive && !moved) loading = false;
	}
}

async function send(): Promise<void> {
	if (busy) return;
	busy = true;
	fault = undefined;
	try {
		await account.login({ login: login.trim(), pass });
		if (alive) globalThis.location.assign(target ?? "/");
	} catch (error) {
		if (alive) fault = error instanceof Fault ? error : new Fault(0, "login");
	} finally {
		pass = "";
		if (alive) busy = false;
	}
}

onMount(() => {
	checked = true;
	void inspect();
	return () => {
		alive = false;
	};
});
</script>

<Rite title="登录" line="使用你的 Ensign 账号继续。">
	{#if loading}
		<Note text="正在确认会话…" live />
	{:else}
		<Form label="登录" submit={send} {busy}>
			<Field
				label="账号"
				name="login"
				autocomplete="username"
				look="float"
				bind:value={login}
				required
				disabled={busy}
			/>
			<Field
				label="密码"
				name="pass"
				autocomplete="current-password"
				kind="password"
				look="float"
				bind:value={pass}
				required
				disabled={busy}
			/>
			<Note text="仅在信任的设备上保持登录。" />
			<Split look="end">
				<Button label={busy ? "正在登录…" : "登录"} submit {busy} />
			</Split>
		</Form>
	{/if}
	{#snippet foot()}
		{#if fault}
			<Note text={fault.message} mood={fault.mood} code={fault.code} live />
		{/if}
		{#if checked && raw !== null && target === null}
			<Note text="返回地址无效，登录后将前往个人中心。" />
		{/if}
		<Split>
			<Link href="/join" label="使用邀请码注册" />
			<Link href="/recover" label="使用恢复码重置密码" />
		</Split>
	{/snippet}
</Rite>
