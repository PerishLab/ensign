<script lang="ts">
import {
	Board,
	Button,
	Modal,
	Note,
	Split,
	Table,
	Text,
} from "@perishlab/design";
import { onMount } from "svelte";
import {
	account,
	Fault,
	type Inventory,
	type Session,
	stamp,
	type Token,
} from "../../../account.ts";
import { forget } from "../../../authorization.ts";
import { login } from "../route.ts";

type Target =
	| { kind: "session"; item: Session }
	| { kind: "token"; item: Token };

let {
	toast,
}: {
	toast: (text: string, mood?: "calm" | "warn") => void;
} = $props();

let data: Inventory = $state({ sessions: [], tokens: [] });
let loading = $state(true);
let fault: Fault | undefined = $state();
let target: Target | undefined = $state();
let issue: Fault | undefined = $state();
let modal = $state(false);
let busy = $state(false);
let alive = true;

$effect(() => {
	if (!modal) issue = undefined;
});

let phase = $derived<"loading" | "error" | "ready">(
	loading && data.sessions.length === 0 && data.tokens.length === 0
		? "loading"
		: fault
			? "error"
			: "ready",
);
let rows = $derived(
	data.sessions.map((item) => [
		`#${item.id}`,
		item.current ? "当前会话" : "其他设备",
		stamp(item.created, "—"),
		stamp(item.expires, "永不过期"),
	]),
);
let keys = $derived(
	data.tokens.map((item) => [
		item.name || "未命名令牌",
		`#${item.id}`,
		item.current ? "当前使用" : "有效",
		stamp(item.created, "—"),
		stamp(item.expires, "永不过期"),
	]),
);

async function load(): Promise<void> {
	loading = true;
	fault = undefined;
	try {
		const next = await account.sessions();
		if (alive) data = next;
	} catch (error) {
		if (!alive) return;
		if (error instanceof Fault && error.status === 401) {
			forget();
			login("/sessions");
			return;
		}
		fault = error instanceof Fault ? error : new Fault(0, "session");
	} finally {
		if (alive) loading = false;
	}
}

function pick(kind: "session" | "token", index: number): void {
	if (kind === "session") {
		const item = data.sessions[index];
		if (!item) return;
		target = { kind, item };
	} else {
		const item = data.tokens[index];
		if (!item) return;
		target = { kind, item };
	}
	issue = undefined;
	modal = true;
}

async function revoke(): Promise<void> {
	if (!target || busy) return;
	const chosen = target;
	issue = undefined;
	busy = true;
	try {
		if (chosen.kind === "session") {
			await account.evict(chosen.item.id);
			if (chosen.item.current) {
				forget();
				globalThis.location.assign("/login");
				return;
			}
			modal = false;
			target = undefined;
			toast(`已撤销会话 #${chosen.item.id}。`, "calm");
		} else {
			await account.revoke(chosen.item.id);
			modal = false;
			target = undefined;
			toast(`已撤销访问令牌「${chosen.item.name || `#${chosen.item.id}`}」。`, "calm");
		}
		await load();
	} catch (error) {
		if (!alive) return;
		if (error instanceof Fault && error.status === 401) {
			forget();
			login("/sessions");
			return;
		}
		if (error instanceof Fault && error.status === 404) {
			modal = false;
			target = undefined;
			toast("目标凭据已不存在，已刷新列表。", "calm");
			await load();
			return;
		}
		const next = error instanceof Fault ? error : new Fault(0, "session");
		issue = next;
		toast(next.message, "warn");
	} finally {
		if (alive) busy = false;
	}
}

onMount(() => {
	void load();
	return () => {
		alive = false;
	};
});
</script>

<Board title="浏览器会话" line="已通过浏览器登录的活跃会话。点击任意一行可撤销对应设备的登录状态。">
	{#snippet side()}
		<Button label="刷新" look="quiet" sign="sync" busy={loading} press={() => void load()} />
	{/snippet}
	<Table
		label="活跃浏览器会话"
		heads={["编号", "状态", "创建时间", "过期时间"]}
		{rows}
		state={phase === "ready" && rows.length === 0 ? "empty" : phase}
		note={fault ? fault.message : "当前没有活跃的浏览器会话。"}
		retry={() => void load()}
		action="重试"
		pick={(index) => pick("session", index)}
	/>
</Board>

<Board title="个人访问令牌" line="供 ensign CLI 或非交互客户端使用的长期访问令牌。点击任意一行可立即吊销。">
	<Table
		label="个人访问令牌"
		heads={["名称", "编号", "状态", "创建时间", "过期时间"]}
		rows={keys}
		state={phase === "ready" && keys.length === 0 ? "empty" : phase}
		note={fault ? fault.message : "暂无个人访问令牌。使用 ensign CLI 登录后将在此显示。"}
		retry={() => void load()}
		action="重试"
		pick={(index) => pick("token", index)}
	/>
</Board>

<Modal
	title={target?.kind === "token" ? "撤销个人访问令牌？" : "撤销浏览器会话？"}
	mood="warn"
	bind:open={modal}
>
	{#if target?.kind === "session"}
		{#if target.item.current}
			<Note
				mood="warn"
				title="即将退出当前设备"
				text="你正在撤销当前浏览器会话。确认后你将立即退出登录，需要重新登录才能继续访问。"
			/>
		{:else}
			<Text>确认撤销会话 #{target.item.id}（创建于 {stamp(target.item.created, "—")}）？该设备上的登录状态将立即失效。</Text>
		{/if}
	{:else if target?.kind === "token"}
		<Text>确认吊销访问令牌「{target.item.name || `#${target.item.id}`}」（编号 #{target.item.id}）？使用该令牌的客户端将立即失去访问权限。</Text>
	{/if}
	{#if issue}
		<Note text={issue.message} mood={issue.mood} code={issue.code} live />
	{/if}
	{#snippet foot()}
		<Split look="end">
			<Button label="取消" look="quiet" halt={busy} press={() => { issue = undefined; modal = false; }} />
			<Button
				label={busy ? "正在撤销…" : target?.kind === "session" && target.item.current ? "确认撤销并退出" : "确认撤销"}
				mood="warn"
				{busy}
				press={() => void revoke()}
			/>
		</Split>
	{/snippet}
</Modal>
