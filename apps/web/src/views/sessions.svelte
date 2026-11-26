<script lang="ts">
import Desk from "../lib/components/desk.svelte";
import Devices from "../lib/components/devices.svelte";

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
	route="/sessions"
	title="会话与令牌"
	line="查看当前账号下的浏览器会话与个人访问令牌，并随时撤销不再信任的凭据。"
	{notes}
	{mood}
>
	{#snippet children()}
		<Devices {toast} />
	{/snippet}
</Desk>
