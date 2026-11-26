<script lang="ts">
import Invites from "../../lib/admin/invites.svelte";
import Desk from "../../lib/components/desk.svelte";

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
	route="/admin/invites"
	title="邀请管理"
	line="签发一次性注册邀请码与加入链接，并随时撤销尚未使用的邀请。"
	{notes}
	{mood}
>
	{#snippet children()}
		<Invites {toast} />
	{/snippet}
</Desk>
