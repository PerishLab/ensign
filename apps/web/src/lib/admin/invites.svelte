<script lang="ts">
import {
	Board,
	Button,
	Copy,
	Drawer,
	Field,
	Form,
	Modal,
	Note,
	Pick,
	Split,
	Table,
	Text,
	Vault,
} from "@perishlab/design";
import { onMount } from "svelte";
import { Fault, stamp } from "../../../account.ts";
import {
	admin,
	type Card,
	fail,
	type Invite,
	type Issued,
	validnote,
} from "../../../admin.ts";
import { forget } from "../../../authorization.ts";
import { login } from "../route.ts";

let {
	toast,
}: {
	toast: (text: string, mood?: "calm" | "warn") => void;
} = $props();

const heads = ["编号", "备注", "创建时间", "过期时间"];
const presets = [
	{ value: "", label: "不过期" },
	{ value: "86400", label: "24 小时" },
	{ value: "604800", label: "7 天" },
	{ value: "2592000", label: "30 天" },
];

let items: Invite[] = $state([]);
let loading = $state(true);
let fault: Fault | undefined = $state();
let drawer = $state(false);
let note = $state("");
let ttl = $state("604800");
let problem: string | undefined = $state();
let issue: Fault | undefined = $state();
let creating = $state(false);
let minted: Issued | undefined = $state();
let shown = $state(false);
let saved = $state(false);
let target: Invite | undefined = $state();
let modal = $state(false);
let revoking = $state(false);
let decline: Fault | undefined = $state();
let seq = 0;
let alive = true;

$effect(() => {
	if (!drawer) {
		problem = undefined;
		issue = undefined;
	}
});

$effect(() => {
	if (!modal) decline = undefined;
});

let phase = $derived<"loading" | "error" | "ready">(
	loading && items.length === 0 ? "loading" : fault ? "error" : "ready",
);
let rows = $derived(
	items.map((item) => [
		`#${item.id}`,
		item.note || "—",
		stamp(item.created, "—"),
		stamp(item.expires, "永不过期"),
	]),
);
let link = $derived(
	minted
		? `${globalThis.location?.origin ?? ""}/join?code=${encodeURIComponent(minted.code)}`
		: "",
);
let secrets = $derived(minted ? [link] : []);

function clear(): void {
	minted = undefined;
	shown = false;
	saved = false;
}

function open(): void {
	note = "";
	ttl = "604800";
	problem = undefined;
	issue = undefined;
	drawer = true;
}

async function load(): Promise<void> {
	const ticket = ++seq;
	loading = true;
	fault = undefined;
	try {
		const list = await admin.invites();
		if (!alive || ticket !== seq) return;
		items = list;
	} catch (error) {
		if (!alive || ticket !== seq) return;
		if (error instanceof Fault && error.status === 401) {
			forget();
			login("/admin/invites");
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
	if (!validnote(note)) {
		problem = "备注最多 120 个字符，且不能包含控制字符。";
		return;
	}
	const payload: Card = {};
	if (note) payload.note = note;
	if (ttl) payload.ttl = Number(ttl);
	creating = true;
	try {
		const created = await admin.invite(payload);
		if (!alive) return;
		minted = created;
		shown = false;
		saved = false;
		drawer = false;
		toast(`已签发邀请 #${created.id}，请立即复制并发送。`, "calm");
		await load();
	} catch (error) {
		if (!alive) return;
		if (error instanceof Fault && error.status === 401) {
			forget();
			login("/admin/invites");
			return;
		}
		const err = error instanceof Fault ? error : fail(0, "invite");
		issue = err;
		toast(err.message, "warn");
	} finally {
		if (alive) creating = false;
	}
}

function pick(index: number): void {
	const chosen = items[index];
	if (!chosen) return;
	target = chosen;
	decline = undefined;
	modal = true;
}

async function cancel(): Promise<void> {
	if (!target || revoking) return;
	const chosen = target;
	decline = undefined;
	revoking = true;
	try {
		await admin.cancel(chosen.id);
		if (!alive) return;
		modal = false;
		target = undefined;
		if (minted?.id === chosen.id) clear();
		toast(`已撤销邀请 #${chosen.id}。`, "calm");
		await load();
	} catch (error) {
		if (!alive) return;
		if (error instanceof Fault && error.status === 401) {
			forget();
			login("/admin/invites");
			return;
		}
		if (error instanceof Fault && error.status === 404) {
			modal = false;
			target = undefined;
			if (minted?.id === chosen.id) clear();
			toast("该邀请已不存在，已刷新列表。", "calm");
			await load();
			return;
		}
		const err = error instanceof Fault ? error : fail(0, "cancel");
		decline = err;
		toast(err.message, "warn");
	} finally {
		if (alive) revoking = false;
	}
}

onMount(() => {
	void load();
	return () => {
		alive = false;
		clear();
	};
});
</script>

{#if minted}
	<Vault
		title={`新签发的邀请 #${minted.id}`}
		line="以下一次性加入链接（含邀请码）仅在此刻显示一次，关闭或离开页面后将无法再次查看。也可直接复制原始邀请码。"
		codes={secrets}
		bind:shown
		bind:held={saved}
		file={`ensign-invite-${minted.id}.txt`}
		check="我已复制或记录该一次性邀请凭据"
		reveal="显示"
		hide="隐藏"
		copy="复制加入链接"
		done="已复制"
		save="下载 .txt"
	>
		<Split look="close">
			<Copy text={minted.code} label="复制邀请码" done="已复制" />
			<Button label="我已保存，关闭显示" halt={!saved} press={clear} />
		</Split>
	</Vault>
{/if}

<Board title="未使用邀请" line="系统仅保存邀请码摘要，列表不显示明文邀请码。点击任意一行可撤销对应邀请。">
	{#snippet side()}
		<Split look="close">
			<Button label="刷新" look="quiet" sign="sync" busy={loading} press={() => void load()} />
			<Button label="签发邀请" sign="takes" press={open} />
		</Split>
	{/snippet}
	<Table
		label="未使用的邀请列表"
		{heads}
		{rows}
		state={phase === "ready" && rows.length === 0 ? "empty" : phase}
		note={fault ? fault.message : "当前没有未使用的邀请。"}
		retry={() => void load()}
		action="重试"
		{pick}
	/>
</Board>

<Drawer title="签发新邀请" line="生成一次性注册邀请码，签发后仅展示一次。" dismiss={!creating} bind:open={drawer}>
	<Form label="签发邀请" submit={create} busy={creating} validate={false}>
		<Field
			label="备注（可选）"
			name="note"
			hint="例如：运维团队新成员"
			help="最多 120 个字符，不能包含控制字符。"
			bind:value={note}
			error={problem}
			disabled={creating}
		/>
		<Pick label="有效期" bind:value={ttl} choices={presets} disabled={creating} />
		{#if issue}
			<Note text={issue.message} mood={issue.mood} code={issue.code} live />
		{/if}
		<Split look="end">
			<Button label="取消" look="quiet" halt={creating} press={() => { drawer = false; }} />
			<Button label={creating ? "正在签发…" : "确认签发"} submit busy={creating} />
		</Split>
	</Form>
</Drawer>

<Modal title="确认撤销邀请？" mood="warn" dismiss={!revoking} bind:open={modal}>
	{#if target}
		<Text>
			确认撤销邀请 #{target.id}{target.note ? `（${target.note}）` : ""}？撤销后持该邀请码的用户将无法注册。
		</Text>
	{/if}
	{#if decline}
		<Note text={decline.message} mood={decline.mood} code={decline.code} live />
	{/if}
	{#snippet foot()}
		<Split look="end">
			<Button label="取消" look="quiet" halt={revoking} press={() => { decline = undefined; modal = false; }} />
			<Button
				label={revoking ? "正在撤销…" : "确认撤销"}
				mood="warn"
				busy={revoking}
				press={() => void cancel()}
			/>
		</Split>
	{/snippet}
</Modal>
