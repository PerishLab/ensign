<script lang="ts">
import { Button, Field, Form, Link, Note, Split, Text } from "@perishlab/design";
import { onMount } from "svelte";
import { account, Fault, validname, validpass } from "../../account.ts";
import Rite from "../lib/components/rite.svelte";

const params = new URLSearchParams(globalThis.location?.search ?? "");
const initial = params.get("code") ?? "";

let code = $state(initial);
let login = $state("");
let name = $state("");
let pass = $state("");
let again = $state("");
let busy = $state(false);
let done = $state(false);
let fault: Fault | undefined = $state();
let issue = $state<{
	code?: string;
	login?: string;
	name?: string;
	pass?: string;
	again?: string;
}>({});
let alive = true;

function check(): boolean {
	const found: typeof issue = {};
	if (!code.trim()) found.code = "请输入邀请码。";
	if (!login.trim()) found.login = "请输入账号名。";
	if (!validname(name)) found.name = "显示名称需为 1 至 120 个字符，且不能包含控制字符。";
	if (!validpass(pass)) found.pass = "密码长度至少需要 8 位字符。";
	if (pass !== again) found.again = "两次输入的密码不一致。";
	issue = found;
	return Object.keys(found).length === 0;
}

async function send(): Promise<void> {
	if (busy) return;
	fault = undefined;
	if (!check()) return;
	busy = true;
	try {
		await account.join({
			code: code.trim(),
			login: login.trim(),
			name: name.trim(),
			pass,
		});
		if (!alive) return;
		code = "";
		done = true;
	} catch (error) {
		if (!alive) return;
		const next = error instanceof Fault ? error : new Fault(0, "join");
		fault = next;
		const field = ({ 404: "code", 409: "login" } as const)[next.status];
		if (field !== undefined) issue = { [field]: next.message };
	} finally {
		pass = "";
		again = "";
		busy = false;
	}
}

onMount(() => {
	if (params.has("code")) history.replaceState(null, "", "/join");
	return () => {
		alive = false;
		code = "";
		pass = "";
		again = "";
	};
});
</script>

<Rite title={done ? "账号已创建" : "接受邀请"} line={done ? "你的 Ensign 身份已准备就绪。" : "输入邀请码并创建你的 Ensign 账号。"}>
	{#if done}
		<Text>账号已成功创建，现在可以使用新账号登录个人中心。</Text>
		<Note text="建议登录后立即前往「安全与恢复」生成并离线保存一次性恢复码，避免遗忘密码后无法找回账号。" />
		<Split look="end">
			<Button label="前往登录" href="/login" />
		</Split>
	{:else}
		<Form label="接受邀请并注册" submit={send} {busy} validate={false}>
			<Field
				label="邀请码"
				name="code"
				look="float"
				bind:value={code}
				error={issue.code}
				required
				disabled={busy}
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
				label="显示名称"
				name="name"
				autocomplete="name"
				look="float"
				bind:value={name}
				error={issue.name}
				required
				disabled={busy}
			/>
			<Field
				label="密码"
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
				label="确认密码"
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
				<Button label={busy ? "正在创建…" : "创建账号"} submit {busy} />
			</Split>
		</Form>
	{/if}
	{#snippet foot()}
		{#if fault}
			<Note text={fault.message} mood={fault.mood} code={fault.code} live />
		{/if}
		<Split>
			<Link href="/login" label="已有账号？返回登录" />
			<Link href="/recover" label="使用恢复码重置密码" />
		</Split>
	{/snippet}
</Rite>
