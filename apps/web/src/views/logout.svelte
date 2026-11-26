<script lang="ts">
import { Button, Form, Line, Link, Note, Split, Text } from "@perishlab/design";
import { onMount } from "svelte";
import { type Actor, account, Fault } from "../../account.ts";
import { forget } from "../../authorization.ts";
import Rite from "../lib/components/rite.svelte";

let who: Actor | undefined = $state();
let loading = $state(true);
let busy = $state(false);
let ended = $state(false);
let fault: Fault | undefined = $state();
let alive = true;

async function inspect(): Promise<void> {
	loading = true;
	fault = undefined;
	try {
		const actor = await account.whoami();
		if (alive) who = actor;
	} catch (error) {
		if (!alive) return;
		if (error instanceof Fault && error.status === 401) {
			forget();
			ended = true;
			return;
		}
		fault = error instanceof Fault ? error : new Fault(0, "read");
	} finally {
		if (alive) loading = false;
	}
}

async function leave(): Promise<void> {
	if (busy) return;
	busy = true;
	fault = undefined;
	try {
		await account.logout();
		if (!alive) return;
		forget();
		who = undefined;
		ended = true;
	} catch (error) {
		if (alive) fault = error instanceof Fault ? error : new Fault(0, "logout");
	} finally {
		if (alive) busy = false;
	}
}

onMount(() => {
	void inspect();
	return () => {
		alive = false;
	};
});
</script>

<Rite title={ended ? "已退出登录" : "退出登录"} line="退出此设备上的 Ensign 会话。">
	{#if loading}
		<Note text="正在确认会话…" live />
	{:else if ended}
		<Text>已安全退出当前设备上的登录会话，你可以放心关闭此页面。</Text>
		<Split look="end">
			<Button label="返回登录" href="/login" />
		</Split>
	{:else}
		<Form label="退出登录" submit={leave} {busy}>
			{#if who}
				<Line name={who.name} meta={who.login} />
			{/if}
			<Text>退出后，再次访问受保护内容或个人中心时需要重新登录。</Text>
			<Split>
				<Link href="/" label="取消" />
				<Button label={busy ? "正在退出…" : "确认退出"} mood="warn" submit {busy} />
			</Split>
		</Form>
	{/if}
	{#snippet foot()}
		{#if fault}
			<Note text={fault.message} mood={fault.mood} code={fault.code} live />
		{/if}
	{/snippet}
</Rite>
