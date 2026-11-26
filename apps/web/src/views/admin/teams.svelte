<script lang="ts">
import Teams from "../../lib/admin/teams.svelte";
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
	route="/admin/teams"
	title="团队与成员"
	line="创建并维护团队目录，随时查看或调整团队成员归属。"
	{notes}
	{mood}
>
	{#snippet children()}
		<Teams {toast} />
	{/snippet}
</Desk>
