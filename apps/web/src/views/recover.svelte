<script lang="ts">
import { Button, Field, Form, Link, Note, Split, Text } from "@perishlab/design";
import { account, Fault, validpass } from "../../account.ts";
import Rite from "../lib/components/rite.svelte";

let login = $state("");
let code = $state("");
let pass = $state("");
let again = $state("");
let busy = $state(false);
let done = $state(false);
let fault: Fault | undefined = $state();
let issue = $state<{
	login?: string;
	code?: string;
	pass?: string;
	again?: string;
}>({});

function check(): boolean {
	const found: typeof issue = {};
	if (!login.trim()) found.login = "请输入账号。";
	if (!code.trim()) found.code = "请输入一次性恢复码。";
	if (!validpass(pass)) found.pass = "新密码长度至少需要 8 位字符。";
	if (pass !== again) found.again = "两次输入的新密码不一致。";
	issue = found;
	return Object.keys(found).length === 0;
}

async function send(): Promise<void> {
	if (busy) return;
	fault = undefined;
	if (!check()) return;
	busy = true;
	try {
		await account.revive({
			login: login.trim(),
			code: code.trim(),
			pass,
		});
		done = true;
	} catch (error) {
		fault = error instanceof Fault ? error : new Fault(0, "revive");
	} finally {
		code = "";
		pass = "";
		again = "";
		busy = false;
	}
}
</script>

<Rite title={done ? "密码已重置" : "恢复账号"} line={done ? "所有旧会话与旧恢复码均已失效。" : "使用离线保存的一次性恢复码重置密码。"}>
	{#if done}
		<Text>密码已成功重置。请使用新密码重新登录。</Text>
		<Note text="原有恢复码已全部作废，登录后请立即在「安全与恢复」页面重新生成并保存新的一组恢复码。" />
		<Split look="end">
			<Button label="前往登录" href="/login" />
		</Split>
	{:else}
		<Form label="使用恢复码重置密码" submit={send} {busy} validate={false}>
			<Note
				mood="warn"
				title="安全提示"
				text="完成重置后，该账号下所有现有登录会话以及其余未使用的恢复码都将立即撤销。"
			/>
			<Field
				label="账号"
				name="login"
				autocomplete="username"
				look="float"
				bind:value={login}
				error={issue.login}
				required
				disabled={busy}
			/>
			<Field
				label="恢复码"
				name="code"
				autocomplete="one-time-code"
				look="float"
				bind:value={code}
				error={issue.code}
				required
				disabled={busy}
			/>
			<Field
				label="新密码"
				name="pass"
				autocomplete="new-password"
				kind="password"
				look="float"
				help="至少 8 位字符。"
				bind:value={pass}
				error={issue.pass}
				required
				disabled={busy}
			/>
			<Field
				label="确认新密码"
				name="again"
				autocomplete="new-password"
				kind="password"
				look="float"
				bind:value={again}
				error={issue.again}
				required
				disabled={busy}
			/>
			<Split look="end">
				<Button label={busy ? "正在重置…" : "重置密码"} mood="warn" submit {busy} />
			</Split>
		</Form>
	{/if}
	{#snippet foot()}
		{#if fault}
			<Note text={fault.message} mood={fault.mood} code={fault.code} live />
		{/if}
		<Split>
			<Link href="/login" label="返回登录" />
			<Link href="/join" label="使用邀请码注册" />
		</Split>
	{/snippet}
</Rite>
