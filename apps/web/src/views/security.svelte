<script lang="ts">
import Desk from "../lib/components/desk.svelte";
import Password from "../lib/components/password.svelte";
import Rescue from "../lib/components/rescue.svelte";

let notes: string[] = $state([]);
let mood: "calm" | "warn" = $state("calm");
let timer: ReturnType<typeof setTimeout> | undefined;

function toast(text: string, next: "calm" | "warn" = "calm"): void {
	mood = next;
	notes = [text];
	if (timer !== undefined) clearTimeout(timer);
	timer = setTimeout(() => {
		notes = [];
	}, 4000);
}
</script>

<Desk
	route="/security"
	title="安全与恢复"
	line="定期更新登录密码，并离线保管一次性恢复码以防账号锁死。"
	{notes}
	{mood}
>
	{#snippet children()}
		<Password {toast} />
		<Rescue {toast} />
	{/snippet}
</Desk>
