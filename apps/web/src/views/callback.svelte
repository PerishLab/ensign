<script lang="ts">
import { Frame, Hero, Link, Note, Sheet, Shell } from "@perishlab/design";
import { onMount } from "svelte";
import { complete, configuration, Refusal } from "../../authorization.ts";

let warning = $state("");

onMount(() => {
	const url = location.href;
	history.replaceState(null, "", "/callback");
	void configuration().then(async config => {
		if (!config) throw new Refusal("暂时无法确认身份，请返回登录重试。");
		const back = await complete(config, { url, storage: sessionStorage, fetcher: fetch });
		history.replaceState(null, "", back);
		dispatchEvent(new PopStateEvent("popstate"));
	}).catch(error => { warning = error instanceof Refusal ? error.message : "暂时无法确认身份，请返回登录重试。"; });
});
</script>

<svelte:head><title>Ensign · 确认身份</title></svelte:head>
<Shell system="google-accounts"><Frame look="center"><Sheet>
	<Hero title="确认身份" line="正在准备你的设置。" mark="ensign" look="brief" />
	{#if warning}<Note text={warning} mood="warn" live /><Link href="/login" label="返回登录" />{:else}<Note text="请稍候…" live />{/if}
</Sheet></Frame></Shell>
