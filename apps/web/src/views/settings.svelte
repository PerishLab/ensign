<script lang="ts">
import { shown, wear } from "../../authorization.ts";
import type { Tone } from "../../tone.ts";
import Desk from "../lib/components/desk.svelte";
import Folio from "../lib/components/folio.svelte";

const initial = shown();
let system: string | undefined = $state(initial?.system);
let mode: Tone | undefined = $state(initial?.tone);

function apply(next: { system?: string; tone?: Tone }): void {
	system = next.system;
	mode = next.tone;
	wear(next);
}

function flip(next: Tone): void {
	mode = next;
	wear({ system, tone: next });
}
</script>

<Desk
	route="/settings"
	title="偏好设置"
	line="快速切换 Ensign 主题与外观，或前往 Folio 全域设置中心管理跨应用设置。"
	system={system ?? "google-workspace"}
	{mode}
	shade={flip}
>
	{#snippet children()}
		<Folio present={apply} />
	{/snippet}
</Desk>
