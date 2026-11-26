<script lang="ts">
import {
	Board,
	Button,
	Field,
	Form,
	Note,
	Section,
	Split,
} from "@perishlab/design";
import { account, Fault, validpass } from "../../../account.ts";
import { forget } from "../../../authorization.ts";
import { login } from "../route.ts";

let {
	toast,
}: {
	toast: (text: string, mood?: "calm" | "warn") => void;
} = $props();

let old = $state("");
let pass = $state("");
let again = $state("");
let busy = $state(false);
let fault: Fault | undefined = $state();
let issue = $state<{ old?: string; pass?: string; again?: string }>({});

function check(): boolean {
	const found: typeof issue = {};
	if (!old) found.old = "请输入当前密码。";
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
		await account.repass({ old, pass });
		toast("登录密码已更新。", "calm");
	} catch (error) {
		if (error instanceof Fault && error.status === 401) {
			forget();
			login("/security");
			return;
		}
		const next = error instanceof Fault ? error : new Fault(0, "repass");
		fault = next;
		if (next.status === 403) issue = { old: next.message };
		toast(next.message, "warn");
	} finally {
		old = "";
		pass = "";
		again = "";
		busy = false;
	}
}
</script>

<Board title="修改密码" line="定期轮换密码以保护账号安全。新密码至少需要 8 位字符。">
	<Form label="修改密码" submit={send} {busy} validate={false}>
		<Section title="密码凭据" line="验证当前密码后即可生效。如需清理其他设备登录，请前往「会话与令牌」。">
			<Field
				label="当前密码"
				name="old"
				kind="password"
				autocomplete="current-password"
				bind:value={old}
				error={issue.old}
				required
				disabled={busy}
			/>
			<Field
				label="新密码"
				name="pass"
				kind="password"
				autocomplete="new-password"
				help="至少 8 位字符。"
				bind:value={pass}
				error={issue.pass}
				required
				disabled={busy}
			/>
			<Field
				label="确认新密码"
				name="again"
				kind="password"
				autocomplete="new-password"
				bind:value={again}
				error={issue.again}
				required
				disabled={busy}
			/>
		</Section>
		{#if fault}
			<Note text={fault.message} mood={fault.mood} code={fault.code} live />
		{/if}
		<Split look="end">
			<Button label={busy ? "正在更新…" : "更新密码"} submit {busy} />
		</Split>
	</Form>
</Board>
