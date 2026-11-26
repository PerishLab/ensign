<script lang="ts">
import { Frame, Hero, Sheet, Shell } from "@perishlab/design";
import { onMount, type Snippet } from "svelte";
import { read, type Tone, watch } from "../../../tone.ts";

let {
	title,
	line,
	children,
	foot,
}: {
	title: string;
	line: string;
	children: Snippet;
	foot?: Snippet;
} = $props();

let tone: Tone = $state(read());

onMount(() => watch((next) => { tone = next; }));
</script>

<svelte:head><title>Ensign · {title}</title></svelte:head>

<Shell system="google-accounts" {tone}>
	<Frame look="center">
		<Sheet look="spread">
			{#snippet lead()}
				<Hero {title} {line} mark="ensign" look="brief" />
			{/snippet}
			{@render children()}
			{#snippet foot()}
				{@render foot?.()}
			{/snippet}
		</Sheet>
	</Frame>
</Shell>
