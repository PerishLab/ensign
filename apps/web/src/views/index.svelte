<script lang="ts">
import Desk from "../lib/components/desk.svelte";
import Profile from "../lib/components/profile.svelte";

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
	route="/"
	title="个人概览"
	line="管理你的 Ensign 身份资料，查看恢复码与当前登录状态。"
	{notes}
	{mood}
>
	{#snippet children(who, sync)}
		<Profile {who} {sync} {toast} />
	{/snippet}
</Desk>
