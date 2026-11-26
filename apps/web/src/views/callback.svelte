<script lang="ts">
import { Link, Note } from "@perishlab/design";
import { onMount } from "svelte";
import { complete, configuration, Refusal } from "../../authorization.ts";
import Rite from "../lib/components/rite.svelte";

let warning = $state("");

onMount(() => {
	const url = location.href;
	history.replaceState(null, "", "/callback");
	void configuration()
		.then(async (config) => {
			if (!config) throw new Refusal("暂时无法确认身份，请返回登录重试。");
			const back = await complete(config, {
				url,
				storage: sessionStorage,
				fetcher: fetch,
			});
			history.replaceState(null, "", back);
			dispatchEvent(new PopStateEvent("popstate"));
		})
		.catch((error) => {
			warning =
				error instanceof Refusal
					? error.message
					: "暂时无法确认身份，请返回登录重试。";
		});
});
</script>

<Rite title="确认身份" line="正在准备你的设置。">
	{#if warning}
		<Note text={warning} mood="warn" live />
		<Link href="/login" label="返回登录" />
	{:else}
		<Note text="请稍候…" live />
	{/if}
</Rite>
