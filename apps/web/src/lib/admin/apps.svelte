<script lang="ts">
import {
	Board,
	Button,
	Drawer,
	Field,
	Form,
	Note,
	Pick,
	Search,
	Section,
	Split,
	Table,
} from "@perishlab/design";
import { onMount } from "svelte";
import { Fault, stamp } from "../../../account.ts";
import {
	type App,
	admin,
	type Craft,
	fail,
	type Mode,
	validapp,
	validname,
	validslug,
} from "../../../admin.ts";
import { forget } from "../../../authorization.ts";
import { login } from "../route.ts";
import Amend from "./amend.svelte";

let {
	toast,
}: {
	toast: (text: string, mood?: "calm" | "warn") => void;
} = $props();

const heads = ["名称", "标识 (slug)", "模式", "状态", "创建时间"];
const filters = [
	{ value: "all", label: "全部" },
	{ value: "oidc", label: "OIDC" },
	{ value: "forward", label: "Forward-Auth" },
	{ value: "resource", label: "Resource" },
];
const modes = [
	{ value: "oidc", label: "OIDC（授权码 + PKCE）" },
	{ value: "forward", label: "Forward-Auth（反向代理鉴权）" },
	{ value: "resource", label: "Resource（RFC 8693 受众）" },
];
const labels: Record<Mode, string> = {
	oidc: "OIDC",
	forward: "Forward-Auth",
	resource: "Resource",
};
const hints: Record<Mode, { home: string; redirect: string }> = {
	oidc: {
		home: "必填，例如 https://app.example（本地允许 http://localhost / http://127.0.0.1）。",
		redirect: "必填，OIDC 回调地址，不能包含查询参数或 #。",
	},
	forward: {
		home: "必填，受保护服务的主页地址（https:// 或本地 localhost/127.0.0.1）。",
		redirect: "可选，若填写须为不含查询参数与 # 的有效地址。",
	},
	resource: {
		home: "可选，非交互受众应用可留空。",
		redirect: "可选，非交互受众应用可留空。",
	},
};

let items: App[] = $state([]);
let loading = $state(true);
let fault: Fault | undefined = $state();
let query = $state("");
let scope = $state("all");
let drawer = $state(false);
let name = $state("");
let slug = $state("");
let mode: Mode = $state("oidc");
let home = $state("");
let redirect = $state("");
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
		if (scope !== "all" && item.mode !== scope) return false;
		const needle = query.trim().toLowerCase();
		if (!needle) return true;
		return (
			item.name.toLowerCase().includes(needle) ||
			item.slug.toLowerCase().includes(needle)
		);
	}),
);
let current = $derived(items.find((item) => item.id === active));
let phase = $derived<"loading" | "error" | "ready">(
	loading && items.length === 0 ? "loading" : fault ? "error" : "ready",
);
let rows = $derived(
	visible.map((item) => [
		item.name,
		item.slug,
		labels[item.mode] ?? item.mode,
		item.barred ? "已停用" : "启用",
		stamp(item.created, "—"),
	]),
);
let guide = $derived(hints[mode]);

function open(): void {
	name = "";
	slug = "";
	mode = "oidc";
	home = "";
	redirect = "";
	problem = undefined;
	issue = undefined;
	drawer = true;
}

function choose(next: string): void {
	if (next === "oidc" || next === "forward" || next === "resource") {
		mode = next;
	}
}

async function load(): Promise<void> {
	const ticket = ++seq;
	loading = true;
	fault = undefined;
	try {
		const list = await admin.apps();
		if (!alive || ticket !== seq) return;
		items = list;
	} catch (error) {
		if (!alive || ticket !== seq) return;
		if (error instanceof Fault && error.status === 401) {
			forget();
			login("/admin/apps");
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
	const clean = slug.trim();
	const site = home.trim();
	const back = redirect.trim();
	if (!validname(name)) {
		problem = "应用名称需为 1 至 120 个字符，且不能包含控制字符。";
		return;
	}
	if (!validslug(clean)) {
		problem = "应用标识 (slug) 需为 1 至 64 位小写字母、数字或连字符，且首尾不可为连字符。";
		return;
	}
	const bad = validapp(mode, site, back);
	if (bad) {
		problem = bad;
		return;
	}
	const payload: Craft = { name: name.trim(), slug: clean, mode };
	if (site) payload.home = site;
	if (back) payload.redirect = back;
	creating = true;
	try {
		const created = await admin.create(payload);
		if (!alive) return;
		drawer = false;
		toast(`已登记应用「${created.name}」（${created.slug}）。`, "calm");
		await load();
	} catch (error) {
		if (!alive) return;
		if (error instanceof Fault && error.status === 401) {
			forget();
			login("/admin/apps");
			return;
		}
		const err = error instanceof Fault ? error : fail(0, "create");
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

<Board title="已登记应用" line="管理 OIDC、Forward-Auth 及 RFC 8693 受众应用。点击任意一行编辑配置或启停应用。">
	{#snippet side()}
		<Split look="close">
			<Button label="刷新" look="quiet" sign="sync" busy={loading} press={() => void load()} />
			<Button label="登记应用" sign="draws" press={open} />
		</Split>
	{/snippet}
	<Split look="apart">
		<Search bind:value={query} hint="按应用名称或 slug 搜索…" label="搜索应用" />
		<Pick label="应用模式" bind:value={scope} choices={filters} />
	</Split>
	<Table
		label="应用登记列表"
		{heads}
		{rows}
		state={phase === "ready" && rows.length === 0 ? "empty" : phase}
		note={fault ? fault.message : query || scope !== "all" ? "没有符合筛选条件的应用。" : "当前没有可见的应用记录。"}
		retry={() => void load()}
		action="重试"
		{pick}
	/>
</Board>

<Drawer title="登记新应用" line="注册新的 OIDC、Forward-Auth 或 Resource 受众应用。" dismiss={!creating} bind:open={drawer}>
	<Form label="登记新应用" submit={create} busy={creating} validate={false}>
		<Section title="基本信息" line="标识 (slug) 与模式在登记后不可修改。">
			<Field
				label="应用名称"
				name="name"
				hint="例如：Codehull"
				help="1 至 120 个字符，不能包含控制字符。"
				bind:value={name}
				required
				disabled={creating}
			/>
			<Field
				label="应用标识 (slug)"
				name="slug"
				hint="例如：codehull-web"
				help="仅限小写字母、数字与连字符（[a-z0-9-]），首尾需为字母或数字。"
				bind:value={slug}
				required
				disabled={creating}
			/>
			<Pick label="模式" value={mode} change={choose} choices={modes} disabled={creating} />
		</Section>
		<Section title="访问地址" line="不同模式对主页地址与回调地址的要求不同。">
			<Field
				label="主页地址 (home)"
				name="home"
				kind="url"
				hint="https://app.example"
				help={guide.home}
				bind:value={home}
				required={mode === "oidc" || mode === "forward"}
				disabled={creating}
			/>
			<Field
				label="回调地址 (redirect)"
				name="redirect"
				kind="url"
				hint="https://app.example/callback"
				help={guide.redirect}
				bind:value={redirect}
				required={mode === "oidc"}
				disabled={creating}
			/>
		</Section>
		{#if problem}
			<Note text={problem} mood="warn" live />
		{/if}
		{#if issue}
			<Note text={issue.message} mood={issue.mood} code={issue.code} live />
		{/if}
		<Split look="end">
			<Button label="取消" look="quiet" halt={creating} press={() => { drawer = false; }} />
			<Button label={creating ? "正在登记…" : "确认登记"} submit busy={creating} />
		</Split>
	</Form>
</Drawer>

<Amend app={current} bind:shown={detail} refresh={load} {toast} />
