<script lang="ts">
import {
	Board,
	Button,
	Drawer,
	Field,
	Form,
	Note,
	Search,
	Split,
	Table,
} from "@perishlab/design";
import { onMount } from "svelte";
import { Fault, stamp } from "../../../account.ts";
import { admin, fail, type Team, validname } from "../../../admin.ts";
import { forget } from "../../../authorization.ts";
import { login } from "../route.ts";
import Roster from "./roster.svelte";

let {
	toast,
}: {
	toast: (text: string, mood?: "calm" | "warn") => void;
} = $props();

const heads = ["团队名称", "编号", "成员数", "创建时间"];

let items: Team[] = $state([]);
let loading = $state(true);
let fault: Fault | undefined = $state();
let query = $state("");
let drawer = $state(false);
let name = $state("");
let problem: string | undefined = $state();
let issue: Fault | undefined = $state();
let creating = $state(false);
let active: number | undefined = $state();
let detail = $state(false);
let seq = 0;
let alive = true;

$effect(() => {
	if (!drawer) {
		problem = undefined;
		issue = undefined;
	}
});

let visible = $derived(
	items.filter((item) => {
		const needle = query.trim().toLowerCase();
		if (!needle) return true;
		return item.name.toLowerCase().includes(needle) || String(item.id).includes(needle);
	}),
);
let current = $derived(items.find((item) => item.id === active));
let phase = $derived<"loading" | "error" | "ready">(
	loading && items.length === 0 ? "loading" : fault ? "error" : "ready",
);
let rows = $derived(
	visible.map((item) => [
		item.name,
		`#${item.id}`,
		`${item.members.length}`,
		stamp(item.created, "—"),
	]),
);

function open(): void {
	name = "";
	problem = undefined;
	issue = undefined;
	drawer = true;
}

async function load(): Promise<void> {
	const ticket = ++seq;
	loading = true;
	fault = undefined;
	try {
		const list = await admin.teams();
		if (!alive || ticket !== seq) return;
		items = list;
	} catch (error) {
		if (!alive || ticket !== seq) return;
		if (error instanceof Fault && error.status === 401) {
			forget();
			login("/admin/teams");
			return;
		}
		fault = error instanceof Fault ? error : fail(0, "read");
	} finally {
		if (alive && ticket === seq) loading = false;
	}
}

async function create(): Promise<void> {
	if (creating) return;
	problem = undefined;
	issue = undefined;
	if (!validname(name)) {
		problem = "团队名称需为 1 至 120 个字符，且不能包含控制字符。";
		return;
	}
	creating = true;
	try {
		const created = await admin.team(name.trim());
		if (!alive) return;
		drawer = false;
		toast(`已创建团队「${created.name}」。`, "calm");
		await load();
		if (alive) {
			active = created.id;
			detail = true;
		}
	} catch (error) {
		if (!alive) return;
		if (error instanceof Fault && error.status === 401) {
			forget();
			login("/admin/teams");
			return;
		}
		const err = error instanceof Fault ? error : fail(0, "team");
		issue = err;
		toast(err.message, "warn");
	} finally {
		if (alive) creating = false;
	}
}

function pick(index: number): void {
	const chosen = visible[index];
	if (!chosen) return;
	active = chosen.id;
	detail = true;
}

onMount(() => {
	void load();
	return () => {
		alive = false;
	};
});
</script>

<Board title="团队目录" line="管理组织内的团队及其成员归属。点击任意团队行查看并增减成员。">
	{#snippet side()}
		<Split look="close">
			<Button label="刷新" look="quiet" sign="sync" busy={loading} press={() => void load()} />
			<Button label="新建团队" sign="places" press={open} />
		</Split>
	{/snippet}
	<Search bind:value={query} hint="按团队名称或编号搜索…" label="搜索团队" />
	<Table
		label="团队列表"
		{heads}
		{rows}
		state={phase === "ready" && rows.length === 0 ? "empty" : phase}
		note={fault ? fault.message : query ? "没有符合筛选条件的团队。" : "当前没有可见的团队。"}
		retry={() => void load()}
		action="重试"
		{pick}
	/>
</Board>

<Drawer title="新建团队" line="创建一个新团队，创建后即可立即添加成员。" dismiss={!creating} bind:open={drawer}>
	<Form label="新建团队" submit={create} busy={creating} validate={false}>
		<Field
			label="团队名称"
			name="name"
			hint="例如：Platform Ops"
			help="1 至 120 个字符，不能包含控制字符。"
			bind:value={name}
			error={problem}
			required
			disabled={creating}
		/>
		{#if issue}
			<Note text={issue.message} mood={issue.mood} code={issue.code} live />
		{/if}
		<Split look="end">
			<Button label="取消" look="quiet" halt={creating} press={() => { drawer = false; }} />
			<Button label={creating ? "正在创建…" : "确认创建"} submit busy={creating} />
		</Split>
	</Form>
</Drawer>

<Roster team={current} bind:shown={detail} refresh={load} {toast} />
