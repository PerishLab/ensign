<script lang="ts">
import Desk from "../lib/components/desk.svelte";
import Folio from "../lib/components/folio.svelte";
import type { Tone } from "../lib/tone.ts";

let system: string | undefined = $state();
let mode: Tone | undefined = $state();

function apply(next: { system?: string; tone?: Tone }): void {
	system = next.system;
	mode = next.tone;
}
</script>

<Desk
	route="/settings"
	title="偏好设置"
	line="管理此应用与跨应用共享的主题与外观偏好。"
	system={system ?? "google-workspace"}
	{mode}
>
	{#snippet children()}
		<Folio present={apply} />
	{/snippet}
</Desk>
