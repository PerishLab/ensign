<script lang="ts">
import {
	Button,
	Drawer,
	Field,
	Form,
	Line,
	Modal,
	Note,
	Section,
	Split,
	Tag,
	Text,
} from "@perishlab/design";
import { onMount } from "svelte";
import { Fault, stamp } from "../../../account.ts";
import {
	type Amend,
	type App,
	admin,
	fail,
	type Mode,
	validapp,
	validname,
} from "../../../admin.ts";
import { forget } from "../../../authorization.ts";
import { login } from "../route.ts";

let {
	app,
	shown = $bindable(false),
	refresh,
	toast,
}: {
	app: App | undefined;
	shown?: boolean;
	refresh: () => Promise<void>;
	toast: (text: string, mood?: "calm" | "warn") => void;
} = $props();

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

let seeded = $state(0);
let name = $state("");
let home = $state("");
let redirect = $state("");
let problem: string | undefined = $state();
let issue: Fault | undefined = $state();
let saving = $state(false);
let modal = $state(false);
let toggling = $state(false);
let decline: Fault | undefined = $state();
let alive = true;

$effect(() => {
	if (shown && app && seeded !== app.id) {
		seeded = app.id;
		name = app.name;
		home = app.home;
		redirect = app.redirect;
		problem = undefined;
		issue = undefined;
	}
	if (!shown) {
		seeded = 0;
		problem = undefined;
		issue = undefined;
	}
});

$effect(() => {
	if (!modal) decline = undefined;
});

function changed(): boolean {
	if (!app) return false;
	if (name.trim() !== app.name) return true;
	if (home.trim() !== app.home) return true;
	return redirect.trim() !== app.redirect;
}

let dirty = $derived(changed());
let guide = $derived(app ? hints[app.mode] : hints.oidc);

function reset(): void {
	if (!app) return;
	name = app.name;
	home = app.home;
	redirect = app.redirect;
	problem = undefined;
	issue = undefined;
}

async function save(): Promise<void> {
	const target = app;
	if (!target || saving || !dirty) return;
	problem = undefined;
	issue = undefined;
	if (!validname(name)) {
		problem = "应用名称需为 1 至 120 个字符，且不能包含控制字符。";
		return;
	}
	const site = home.trim();
	const back = redirect.trim();
	const bad = validapp(target.mode, site, back);
	if (bad) {
		problem = bad;
		return;
	}
	const payload: Amend = {};
	if (name.trim() !== target.name) payload.name = name.trim();
	if (site !== target.home) payload.home = site;
	if (back !== target.redirect) payload.redirect = back;
	saving = true;
	try {
		const updated = await admin.update(target.id, payload);
		if (!alive) return;
		if (app?.id === target.id) {
			name = updated.name;
			home = updated.home;
			redirect = updated.redirect;
		}
		toast(`已更新应用「${updated.name}」。`, "calm");
		await refresh();
	} catch (error) {
		if (!alive) return;
		if (error instanceof Fault && error.status === 401) {
			forget();
			login("/admin/apps");
			return;
		}
		const err = error instanceof Fault ? error : fail(0, "update");
		if (app?.id === target.id) issue = err;
		toast(err.message, "warn");
	} finally {
		if (alive) saving = false;
	}
}

async function toggle(): Promise<void> {
	const target = app;
	if (!target || toggling) return;
	const next = !target.barred;
	decline = undefined;
	toggling = true;
	try {
		const updated = await admin.update(target.id, { barred: next });
		if (!alive) return;
		if (app?.id === target.id) modal = false;
		toast(
			next
				? `已停用应用「${updated.name}」。`
				: `已重新启用应用「${updated.name}」。`,
			"calm",
		);
		await refresh();
	} catch (error) {
		if (!alive) return;
		if (error instanceof Fault && error.status === 401) {
			forget();
			login("/admin/apps");
			return;
		}
		const err = error instanceof Fault ? error : fail(0, "update");
		if (app?.id === target.id) decline = err;
		toast(err.message, "warn");
	} finally {
		if (alive) toggling = false;
	}
}

onMount(() => () => {
	alive = false;
});
</script>

<Drawer
	title={app ? app.name : "应用详情"}
	line={app ? `标识 ${app.slug} · ${labels[app.mode]}` : undefined}
	dismiss={!saving && !toggling}
	bind:open={shown}
>
	{#if app}
		<Line name="状态" meta={app.barred ? "已停用，所有授权与令牌交换均被阻止" : "当前正常启用"}>
			<Tag text={app.barred ? "已停用" : "启用"} mood={app.barred ? "warn" : "calm"} />
		</Line>
		<Line name="标识 (slug)" meta={app.slug} />
		<Line name="模式" meta={labels[app.mode]} />
		<Line name="创建时间" meta={stamp(app.created, "—")} />
		<Form label="编辑应用配置" submit={save} busy={saving} validate={false}>
			<Section title="可编辑配置" line="应用标识与模式不可变更，可更新展示名称及地址。">
				<Field
					label="应用名称"
					name="name"
					help="1 至 120 个字符，不能包含控制字符。"
					bind:value={name}
					required
					disabled={saving}
				/>
				<Field
					label="主页地址 (home)"
					name="home"
					kind="url"
					help={guide.home}
					bind:value={home}
					required={app.mode === "oidc" || app.mode === "forward"}
					disabled={saving}
				/>
				<Field
					label="回调地址 (redirect)"
					name="redirect"
					kind="url"
					help={guide.redirect}
					bind:value={redirect}
					required={app.mode === "oidc"}
					disabled={saving}
				/>
			</Section>
			{#if problem}
				<Note text={problem} mood="warn" live />
			{/if}
			{#if issue}
				<Note text={issue.message} mood={issue.mood} code={issue.code} live />
			{/if}
			<Split look="end">
				<Button label="重置" look="quiet" halt={saving || !dirty} press={reset} />
				<Button label={saving ? "正在保存…" : "保存更改"} submit busy={saving} halt={!dirty} />
			</Split>
		</Form>
	{/if}
	{#snippet foot()}
		<Split look="apart">
			{#if app}
				<Button
					label={app.barred ? "重新启用应用" : "停用应用"}
					mood="warn"
					look="quiet"
					halt={saving || toggling}
					press={() => { decline = undefined; modal = true; }}
				/>
			{/if}
			<Button label="关闭" look="quiet" halt={saving || toggling} press={() => { shown = false; }} />
		</Split>
	{/snippet}
</Drawer>

<Modal
	title={app?.barred ? "确认重新启用应用？" : "确认停用应用？"}
	mood="warn"
	dismiss={!toggling}
	bind:open={modal}
>
	{#if app}
		<Text>
			{app.barred
				? `确认重新启用应用「${app.name}」（${app.slug}）？启用后该应用将恢复正常的认证与令牌签发能力。`
				: `确认停用应用「${app.name}」（${app.slug}）？停用后将立即阻止该应用的 OIDC authorize / token / refresh 流程，并拒绝以其为目标受众的 RFC 8693 令牌交换。`}
		</Text>
	{/if}
	{#if decline}
		<Note text={decline.message} mood={decline.mood} code={decline.code} live />
	{/if}
	{#snippet foot()}
		<Split look="end">
			<Button label="取消" look="quiet" halt={toggling} press={() => { decline = undefined; modal = false; }} />
			<Button
				label={toggling ? "正在提交…" : app?.barred ? "确认重新启用" : "确认停用"}
				mood="warn"
				busy={toggling}
				press={() => void toggle()}
			/>
		</Split>
	{/snippet}
</Modal>
