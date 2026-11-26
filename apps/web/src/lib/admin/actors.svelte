<script lang="ts">
import {
	Board,
	Button,
	Copy,
	Drawer,
	Line,
	Modal,
	Note,
	Pick,
	Search,
	Split,
	Table,
	Tag,
	Text,
} from "@perishlab/design";
import { onMount } from "svelte";
import { Fault, stamp } from "../../../account.ts";
import { admin, type Entry, fail } from "../../../admin.ts";
import { forget } from "../../../authorization.ts";
import { login } from "../route.ts";

let {
	toast,
}: {
	toast: (text: string, mood?: "calm" | "warn") => void;
} = $props();

const heads = ["显示名", "账号", "类型", "状态", "团队", "创建时间"];
const filters = [
	{ value: "all", label: "全部" },
	{ value: "user", label: "用户" },
	{ value: "svc", label: "服务" },
	{ value: "barred", label: "已停用" },
];

let items: Entry[] = $state([]);
let loading = $state(true);
let fault: Fault | undefined = $state();
let query = $state("");
let scope = $state("all");
let column = $state("创建时间");
let order = $state<"asc" | "desc">("desc");
let current: Entry | undefined = $state();
let drawer = $state(false);
let modal = $state(false);
let busy = $state(false);
let issue: Fault | undefined = $state();
let notice: Fault | undefined = $state();
let listseq = 0;
let readseq = 0;
let alive = true;

$effect(() => {
	if (!modal) issue = undefined;
});

function label(entry: Entry): string {
	return entry.name || entry.login || entry.sub;
}

const sorters: Record<string, (item: Entry) => string | number> = {
	显示名: (item) => label(item),
	账号: (item) => item.login ?? "",
	类型: (item) => item.kind,
	状态: (item) => (item.barred ? 1 : 0),
	团队: (item) => item.teams.join(","),
	创建时间: (item) => item.created,
};

function compare(left: Entry, right: Entry): number {
	const dir = order === "asc" ? 1 : -1;
	const pick = sorters[column] ?? sorters.创建时间;
	const first = pick(left);
	const second = pick(right);
	if (typeof first === "number" && typeof second === "number") {
		return (first - second) * dir;
	}
	return String(first).localeCompare(String(second), "zh-CN") * dir;
}

function matches(entry: Entry): boolean {
	if (scope === "user" && entry.kind !== "user") return false;
	if (scope === "svc" && entry.kind !== "svc") return false;
	if (scope === "barred" && !entry.barred) return false;
	const needle = query.trim().toLowerCase();
	if (!needle) return true;
	const pool = [entry.name ?? "", entry.login ?? "", entry.sub].join(" ");
	return pool.toLowerCase().includes(needle);
}

let visible = $derived(items.filter(matches).sort(compare));
let phase = $derived<"loading" | "error" | "ready">(
	loading && items.length === 0 ? "loading" : fault ? "error" : "ready",
);
let rows = $derived(
	visible.map((entry) => [
		label(entry),
		entry.login ?? "—",
		entry.kind === "svc" ? "服务" : "用户",
		entry.barred ? "已停用" : "正常",
		entry.teams.length > 0 ? entry.teams.join("、") : "—",
		stamp(entry.created, "—"),
	]),
);

function reorder(head: string): void {
	if (column === head) order = order === "asc" ? "desc" : "asc";
	else {
		column = head;
		order = "asc";
	}
}

async function load(): Promise<void> {
	const ticket = ++listseq;
	loading = true;
	fault = undefined;
	try {
		const list = await admin.actors();
		if (!alive || ticket !== listseq) return;
		items = list;
	} catch (error) {
		if (!alive || ticket !== listseq) return;
		if (error instanceof Fault && error.status === 401) {
			forget();
			login("/admin/actors");
			return;
		}
		fault = error instanceof Fault ? error : fail(0, "read");
	} finally {
		if (alive && ticket === listseq) loading = false;
	}
}

async function inspect(index: number): Promise<void> {
	const chosen = visible[index];
	if (!chosen) return;
	const ticket = ++readseq;
	current = chosen;
	notice = undefined;
	drawer = true;
	try {
		const fresh = await admin.actor(chosen.id);
		if (!alive || ticket !== readseq) return;
		if (current?.id !== chosen.id) return;
		current = fresh;
	} catch (error) {
		if (!alive || ticket !== readseq) return;
		if (current?.id !== chosen.id) return;
		if (error instanceof Fault && error.status === 401) {
			forget();
			login("/admin/actors");
			return;
		}
		notice = error instanceof Fault ? error : fail(0, "read");
	}
}

function ask(): void {
	if (!current) return;
	issue = undefined;
	modal = true;
}

async function toggle(): Promise<void> {
	const target = current;
	if (!target || busy) return;
	const next = !target.barred;
	issue = undefined;
	busy = true;
	try {
		const updated = await admin.bar(target.id, next);
		if (!alive) return;
		if (current?.id === target.id) {
			current = updated;
			modal = false;
		}
		toast(
			next
				? `已停用账号「${label(updated)}」。`
				: `已恢复账号「${label(updated)}」。`,
			"calm",
		);
		await load();
	} catch (error) {
		if (!alive) return;
		if (error instanceof Fault && error.status === 401) {
			forget();
			login("/admin/actors");
			return;
		}
		const err = error instanceof Fault ? error : fail(0, "bar");
		if (current?.id === target.id) issue = err;
		toast(err.message, "warn");
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

<Board title="账号目录" line="列出当前账号有权查看的用户与服务账号。点击任意一行查看详情并管理启停状态。">
	{#snippet side()}
		<Button label="刷新" look="quiet" sign="sync" busy={loading} press={() => void load()} />
	{/snippet}
	<Split look="apart">
		<Search bind:value={query} hint="按显示名、账号或 sub 搜索…" label="搜索账号" />
		<Pick label="状态与类型" bind:value={scope} choices={filters} />
	</Split>
	<Table
		label="账号与服务列表"
		{heads}
		{rows}
		sort={column}
		{order}
		{reorder}
		state={phase === "ready" && rows.length === 0 ? "empty" : phase}
		note={fault ? fault.message : query || scope !== "all" ? "没有符合筛选条件的账号。" : "当前没有可见的账号记录。"}
		retry={() => void load()}
		action="重试"
		pick={(index) => void inspect(index)}
	/>
</Board>

<Drawer
	title={current ? label(current) : "账号详情"}
	line={current ? `编号 #${current.id} · ${current.kind === "svc" ? "服务账号" : "用户账号"}` : undefined}
	dismiss={!busy}
	bind:open={drawer}
>
	{#if current}
		<Line name="编号" meta={`#${current.id}`} />
		<Line name="主体标识 (sub)" meta={current.sub}>
			<Copy text={current.sub} label="复制" done="已复制" />
		</Line>
		<Line name="账号" meta={current.login ?? "—"} />
		<Line name="显示名" meta={current.name ?? "—"} />
		<Line name="类型" meta={current.kind === "svc" ? "服务 (svc)" : "用户 (user)"} />
		<Line name="状态" meta={current.barred ? "当前处于停用状态，无法签发新会话" : "当前正常可用"}>
			<Tag text={current.barred ? "已停用" : "正常"} mood={current.barred ? "warn" : "calm"} />
		</Line>
		<Line name="创建时间" meta={stamp(current.created, "—")} />
		<Line name="所属团队" meta={current.teams.length > 0 ? `共 ${current.teams.length} 个团队` : "未加入任何团队"} />
		{#if current.teams.length > 0}
			<Split look="close">
				{#each current.teams as name (name)}
					<Tag text={name} look="quiet" />
				{/each}
			</Split>
		{/if}
		{#if notice}
			<Note text={notice.message} mood={notice.mood} code={notice.code} live />
		{/if}
	{/if}
	{#snippet foot()}
		<Split look="end">
			<Button label="关闭" look="quiet" halt={busy} press={() => { drawer = false; }} />
			{#if current}
				<Button label={current.barred ? "恢复账号" : "停用账号"} mood="warn" halt={busy} press={ask} />
			{/if}
		</Split>
	{/snippet}
</Drawer>

<Modal
	title={current?.barred ? "确认恢复该账号？" : "确认停用该账号？"}
	mood="warn"
	dismiss={!busy}
	bind:open={modal}
>
	{#if current}
		<Text>
			{current.barred
				? `确认恢复「${label(current)}」（#${current.id}）？恢复后该账号将可再次登录与使用服务。`
				: `确认停用「${label(current)}」（#${current.id}）？停用后该账号将无法登录、刷新或继续获取凭据。`}
		</Text>
	{/if}
	{#if issue}
		<Note text={issue.message} mood={issue.mood} code={issue.code} live />
	{/if}
	{#snippet foot()}
		<Split look="end">
			<Button label="取消" look="quiet" halt={busy} press={() => { issue = undefined; modal = false; }} />
			<Button
				label={busy ? "正在提交…" : current?.barred ? "确认恢复" : "确认停用"}
				mood="warn"
				{busy}
				press={() => void toggle()}
			/>
		</Split>
	{/snippet}
</Modal>
