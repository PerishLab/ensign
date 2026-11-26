<script lang="ts">
import Apps from "../../lib/admin/apps.svelte";
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
	route="/admin/apps"
	title="应用登记"
	line="登记并维护下游 OIDC、Forward-Auth 及 RFC 8693 受众应用。"
	{notes}
	{mood}
>
	{#snippet children()}
		<Apps {toast} />
	{/snippet}
</Desk>
