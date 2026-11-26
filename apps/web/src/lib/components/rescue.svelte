<script lang="ts">
import {
	Board,
	Button,
	Line,
	Modal,
	Note,
	Split,
	Tag,
	Text,
	Vault,
} from "@perishlab/design";
import { onMount } from "svelte";
import { account, Fault, type Rescue, stamp } from "../../../account.ts";
import { forget } from "../../../authorization.ts";
import { login } from "../route.ts";

let {
	toast,
}: {
	toast: (text: string, mood?: "calm" | "warn") => void;
} = $props();

let status: Rescue | undefined = $state();
let loading = $state(true);
let minting = $state(false);
let modal = $state(false);
let fault: Fault | undefined = $state();
let codes: string[] = $state([]);
let shown = $state(false);
let saved = $state(false);
let alive = true;

async function load(): Promise<void> {
	loading = true;
	fault = undefined;
	try {
		const got = await account.rescue();
		if (alive) status = got;
	} catch (error) {
		if (!alive) return;
		if (error instanceof Fault && error.status === 401) {
			forget();
			login("/security");
			return;
		}
		fault = error instanceof Fault ? error : new Fault(0, "rescue");
	} finally {
		if (alive) loading = false;
	}
}

function clear(): void {
	codes = [];
	shown = false;
	saved = false;
}

function request(): void {
	if (minting || !status) return;
	if (status.count > 0) {
		modal = true;
		return;
	}
	void generate();
}

async function generate(): Promise<void> {
	if (minting) return;
	modal = false;
	minting = true;
	fault = undefined;
	try {
		const minted = await account.mint();
		if (!alive) return;
		codes = minted.codes;
		shown = false;
		saved = false;
		toast("已生成新的恢复码，请立即离线保存。", "calm");
		await load();
	} catch (error) {
		if (!alive) return;
		if (error instanceof Fault && error.status === 401) {
			forget();
			login("/security");
			return;
		}
		const next = error instanceof Fault ? error : new Fault(0, "rescue");
		fault = next;
		toast(next.message, "warn");
	} finally {
		if (alive) minting = false;
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

<Board title="一次性恢复码" line="在遗忘密码时用于自助重置密码。每次生成都会产生 3 组新恢复码，并立即作废旧恢复码。">
	{#snippet side()}
		<Button
			label={minting ? "正在生成…" : status && status.count > 0 ? "重新生成恢复码" : "生成恢复码"}
			look={status && status.count > 0 ? "quiet" : "solid"}
			mood={status && status.count > 0 ? "warn" : "calm"}
			busy={minting}
			halt={loading || !status}
			press={request}
		/>
	{/snippet}

	{#if loading && !status}
		<Note text="正在读取恢复码状态…" live />
	{:else if fault && !status}
		<Note mood={fault.mood} code={fault.code} text={fault.message} live>
			<Button label="重试" look="quiet" sign="sync" press={() => void load()} />
		</Note>
	{:else if status}
		<Line
			name={status.count > 0 ? `当前可用：${status.count} 组恢复码` : "当前没有可用恢复码"}
			meta={status.created !== null ? `最近生成时间：${stamp(status.created, "—")}` : "建议立即生成并离线保存"}
		>
			<Tag text={status.count > 0 ? "已就绪" : "未配置"} mood={status.count > 0 ? "calm" : "warn"} />
		</Line>
	{/if}

	{#if fault && status}
		<Note mood={fault.mood} code={fault.code} text={fault.message} live />
	{/if}

	{#if codes.length > 0}
		<Vault
			title="新生成的一次性恢复码"
			line="以下恢复码仅在此刻显示一次，关闭或离开页面后将无法再次查看。请立即复制或下载并离线保管。"
			{codes}
			bind:shown
			bind:held={saved}
			file="ensign-rescue-codes.txt"
			check="我已将这批恢复码安全保存至离线位置"
			reveal="显示"
			hide="隐藏"
			copy="复制全部"
			done="已复制"
			save="下载 .txt"
		>
			<Button label="我已保存，关闭显示" halt={!saved} press={clear} />
		</Vault>
	{/if}
</Board>

<Modal title="重新生成恢复码？" mood="warn" bind:open={modal}>
	<Text>当前仍有 {status?.count ?? 0} 组未使用的恢复码。重新生成将立即作废所有旧恢复码，并生成 3 组新恢复码。</Text>
	{#snippet foot()}
		<Split look="end">
			<Button label="取消" look="quiet" press={() => { modal = false; }} />
			<Button label={minting ? "正在生成…" : "确认重新生成"} mood="warn" busy={minting} press={() => void generate()} />
		</Split>
	{/snippet}
</Modal>
