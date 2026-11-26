<script lang="ts">
import {
	Board,
	Button,
	Card,
	Copy,
	Face,
	Field,
	Form,
	Grid,
	Line,
	Note,
	Section,
	Split,
	Tag,
} from "@perishlab/design";
import { onMount } from "svelte";
import {
	type Actor,
	account,
	Fault,
	type Inventory,
	type Rescue,
	stamp,
	validname,
} from "../../../account.ts";
import { forget } from "../../../authorization.ts";
import { login, visit } from "../route.ts";

let {
	who,
	sync,
	toast,
}: {
	who: Actor;
	sync: (next: Actor) => void;
	toast: (text: string, mood?: "calm" | "warn") => void;
} = $props();

let draft = $state("");
let seeded = $state("");
let busy = $state(false);
let issue: string | undefined = $state();
let fault: Fault | undefined = $state();
let backup: Rescue | undefined = $state();
let fleet: Inventory | undefined = $state();
let status = $state<"loading" | "ready" | "error">("loading");
let alive = true;

$effect(() => {
	if (who.name !== seeded) {
		draft = who.name;
		seeded = who.name;
	}
});

let dirty = $derived(draft.trim() !== who.name);

async function load(): Promise<void> {
	status = "loading";
	try {
		const [got, held] = await Promise.all([
			account.rescue(),
			account.sessions(),
		]);
		if (!alive) return;
		backup = got;
		fleet = held;
		status = "ready";
	} catch (error) {
		if (!alive) return;
		if (error instanceof Fault && error.status === 401) {
			forget();
			login("/");
			return;
		}
		status = "error";
	}
}

function reset(): void {
	draft = who.name;
	issue = undefined;
	fault = undefined;
}

async function save(): Promise<void> {
	if (busy) return;
	issue = undefined;
	fault = undefined;
	if (!validname(draft)) {
		issue = "显示名称需为 1 至 120 个字符，且不能包含控制字符。";
		return;
	}
	busy = true;
	try {
		const next = await account.rename(draft.trim());
		draft = next.name;
		seeded = next.name;
		sync(next);
		toast("个人资料已更新。", "calm");
	} catch (error) {
		if (error instanceof Fault && error.status === 401) {
			forget();
			login("/");
			return;
		}
		const next = error instanceof Fault ? error : new Fault(0, "profile");
		fault = next;
		if (next.status === 400) issue = next.message;
		toast(next.message, "warn");
	} finally {
		busy = false;
	}
}

onMount(() => {
	void load();
	return () => {
		alive = false;
	};
});
</script>

<Grid cols={3}>
	<Card title="我的身份">
		<Line name={who.name} meta={who.login}>
			<Face name={who.name} />
		</Line>
		<Line name="主体标识 (sub)" meta={who.sub}>
			<Copy text={who.sub} label="复制" done="已复制" />
		</Line>
	</Card>
	<Card title="恢复码状态">
		{#if status === "loading"}
			<Note text="正在读取恢复码状态…" />
		{:else if status === "error" || !backup}
			<Note text="暂时无法读取恢复码状态。" mood="warn" />
		{:else}
			<Line
				name={backup.count > 0 ? `剩余 ${backup.count} 组可用` : "尚未配置恢复码"}
				meta={backup.created !== null ? `最近生成：${stamp(backup.created, "—")}` : "建议立即生成并离线保存"}
			>
				<Tag text={backup.count > 0 ? "已保护" : "需配置"} mood={backup.count > 0 ? "calm" : "warn"} />
			</Line>
		{/if}
		<Split look="end">
			<Button label="管理恢复码" look="quiet" sign="next" press={() => visit("/security")} />
		</Split>
	</Card>
	<Card title="活跃会话与令牌">
		{#if status === "loading"}
			<Note text="正在统计活跃凭据…" />
		{:else if status === "error" || !fleet}
			<Note text="暂时无法读取会话列表。" mood="warn" />
		{:else}
			<Line
				name={`${fleet.sessions.length} 个浏览器会话`}
				meta={`${fleet.tokens.length} 个个人访问令牌`}
			>
				<Tag text="活跃中" mood="calm" />
			</Line>
		{/if}
		<Split look="end">
			<Button label="查看详情" look="quiet" sign="next" press={() => visit("/sessions")} />
		</Split>
	</Card>
</Grid>

<Board title="个人资料" line="更新在 Ensign 及下游应用中展示的个人名称。">
	<Form label="更新个人资料" submit={save} {busy} validate={false}>
		<Section title="身份信息" line="账号名与主体标识由系统维护；显示名称支持随时修改。">
			<Field
				label="显示名称"
				name="name"
				autocomplete="name"
				help="1 至 120 个字符，不能包含控制字符。"
				bind:value={draft}
				error={issue}
				required
				disabled={busy}
			/>
			<Field label="账号" name="login" value={who.login} readonly />
			<Field label="主体标识 (sub)" name="sub" value={who.sub} readonly />
		</Section>
		{#if fault}
			<Note text={fault.message} mood={fault.mood} code={fault.code} live />
		{/if}
		<Split look="end">
			<Button label="重置" look="quiet" halt={busy || !dirty} press={reset} />
			<Button label={busy ? "正在保存…" : "保存更改"} submit {busy} halt={!dirty} />
		</Split>
	</Form>
</Board>
