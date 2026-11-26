<script lang="ts">
import {
	Button,
	Drawer,
	Form,
	Line,
	Modal,
	Note,
	Pick,
	Section,
	Split,
	Table,
	Text,
} from "@perishlab/design";
import { onMount } from "svelte";
import { Fault, stamp } from "../../../account.ts";
import {
	admin,
	type Entry,
	fail,
	type Member,
	type Team,
} from "../../../admin.ts";
import { forget } from "../../../authorization.ts";
import { login } from "../route.ts";

let {
	team,
	shown = $bindable(false),
	refresh,
	toast,
}: {
	team: Team | undefined;
	shown?: boolean;
	refresh: () => Promise<void>;
	toast: (text: string, mood?: "calm" | "warn") => void;
} = $props();

const heads = ["显示名", "账号", "类型", "状态"];

let catalog: Entry[] = $state([]);
let loaded = $state(false);
let loading = $state(false);
let fault: Fault | undefined = $state();
let chosen = $state("");
let adding = $state(false);
let issue: Fault | undefined = $state();
let target: Member | undefined = $state();
let modal = $state(false);
let removing = $state(false);
let decline: Fault | undefined = $state();
let seq = 0;
let alive = true;

function display(item: {
	name: string | null;
	login: string | null;
	sub: string;
}): string {
	return item.name || item.login || item.sub;
}

let taken = $derived(new Set((team?.members ?? []).map((item) => item.id)));
let choices = $derived(
	catalog
		.filter((item) => !taken.has(item.id))
		.map((item) => ({
			value: String(item.id),
			label: `${display(item)}${item.login ? ` (${item.login})` : ""} · ${item.kind === "svc" ? "服务" : "用户"}`,
		})),
);
let rows = $derived(
	(team?.members ?? []).map((item) => [
		display(item),
		item.login ?? "—",
		item.kind === "svc" ? "服务" : "用户",
		item.barred ? "已停用" : "正常",
	]),
);
let blocked = $derived(!loaded || fault !== undefined || !chosen);

$effect(() => {
	const id = team?.id;
	if (shown && id !== undefined) {
		issue = undefined;
		void load();
	}
});

$effect(() => {
	if (!modal) decline = undefined;
});

$effect(() => {
	if (choices.length === 0) {
		chosen = "";
	} else if (!choices.some((item) => item.value === chosen)) {
		chosen = choices[0].value;
	}
});

async function load(): Promise<void> {
	const ticket = ++seq;
	loading = true;
	fault = undefined;
	try {
		const list = await admin.actors();
		if (!alive || ticket !== seq) return;
		catalog = list;
		loaded = true;
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

async function enroll(): Promise<void> {
	const group = team;
	if (!group || !chosen || adding) return;
	issue = undefined;
	adding = true;
	try {
		await admin.enroll(group.id, Number(chosen));
		if (!alive) return;
		toast(`已向团队「${group.name}」添加成员。`, "calm");
		await refresh();
	} catch (error) {
		if (!alive) return;
		if (error instanceof Fault && error.status === 401) {
			forget();
			login("/admin/teams");
			return;
		}
		const err = error instanceof Fault ? error : fail(0, "enroll");
		if (team?.id === group.id) issue = err;
		toast(err.message, "warn");
	} finally {
		if (alive) adding = false;
	}
}

function pick(index: number): void {
	const item = team?.members[index];
	if (!item) return;
	target = item;
	decline = undefined;
	modal = true;
}

async function eject(): Promise<void> {
	const group = team;
	const picked = target;
	if (!group || !picked || removing) return;
	decline = undefined;
	removing = true;
	try {
		await admin.eject(group.id, picked.tie);
		if (!alive) return;
		if (team?.id === group.id && target?.tie === picked.tie) {
			modal = false;
			target = undefined;
		}
		toast(`已将「${display(picked)}」移出团队「${group.name}」。`, "calm");
		await refresh();
	} catch (error) {
		if (!alive) return;
		if (error instanceof Fault && error.status === 401) {
			forget();
			login("/admin/teams");
			return;
		}
		if (error instanceof Fault && error.status === 404) {
			if (team?.id === group.id && target?.tie === picked.tie) {
				modal = false;
				target = undefined;
			}
			toast("该成员关系已不存在，已刷新团队列表。", "calm");
			await refresh();
			return;
		}
		const err = error instanceof Fault ? error : fail(0, "eject");
		if (team?.id === group.id) decline = err;
		toast(err.message, "warn");
	} finally {
		if (alive) removing = false;
	}
}

onMount(() => () => {
	alive = false;
});
</script>

<Drawer
	title={team ? team.name : "团队详情"}
	line={team ? `编号 #${team.id} · 创建于 ${stamp(team.created, "—")}` : undefined}
	dismiss={!adding && !removing}
	bind:open={shown}
>
	{#if team}
		<Line name="成员数量" meta={`当前共 ${team.members.length} 名成员；点击成员行可移出团队`} />
		<Table
			label="团队成员列表"
			{heads}
			{rows}
			state={rows.length === 0 ? "empty" : "ready"}
			note="当前团队暂无成员，可在下方添加。"
			{pick}
		/>
		<Form label="添加团队成员" submit={enroll} busy={adding} validate={false}>
			<Section title="添加成员" line="从当前目录中选择尚未加入该团队的账号。">
				{#if loading && !loaded}
					<Note text="正在加载可选账号…" live />
				{:else if fault}
					<Note text={fault.message} mood={fault.mood} code={fault.code} live>
						<Button label="重试加载账号" look="quiet" sign="sync" press={() => void load()} />
					</Note>
				{:else if loaded && choices.length === 0}
					<Note text="当前没有可添加的新成员。" />
				{:else if loaded}
					<Pick label="选择账号" bind:value={chosen} {choices} disabled={adding} />
				{/if}
			</Section>
			{#if issue}
				<Note text={issue.message} mood={issue.mood} code={issue.code} live />
			{/if}
			<Split look="end">
				<Button
					label={adding ? "正在添加…" : "添加成员"}
					submit
					busy={adding}
					halt={blocked}
				/>
			</Split>
		</Form>
	{/if}
	{#snippet foot()}
		<Split look="end">
			<Button label="关闭" look="quiet" halt={adding || removing} press={() => { shown = false; }} />
		</Split>
	{/snippet}
</Drawer>

<Modal title="确认移出团队成员？" mood="warn" dismiss={!removing} bind:open={modal}>
	{#if team && target}
		<Text>
			确认将「{display(target)}」移出团队「{team.name}」？移出后其通过该团队继承的授权将不再有效。
		</Text>
	{/if}
	{#if decline}
		<Note text={decline.message} mood={decline.mood} code={decline.code} live />
	{/if}
	{#snippet foot()}
		<Split look="end">
			<Button label="取消" look="quiet" halt={removing} press={() => { decline = undefined; modal = false; }} />
			<Button
				label={removing ? "正在移出…" : "确认移出"}
				mood="warn"
				busy={removing}
				press={() => void eject()}
			/>
		</Split>
	{/snippet}
</Modal>
