<script lang="ts">
import Actors from "../../lib/admin/actors.svelte";
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
	route="/admin/actors"
	title="账号与服务"
	line="查看目录中的用户与服务账号，核对所属团队并管理启停状态。"
	{notes}
	{mood}
>
	{#snippet children()}
		<Actors {toast} />
	{/snippet}
</Desk>
