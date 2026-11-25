<script lang="ts">
import { Preference } from "@perishlab/design";
import type { Scope, View } from "../../../preferences.ts";

let {
	view,
	level,
	look,
	busy,
	error,
	change,
	restore,
}: {
	view: View;
	level: Scope;
	look: "line" | "list";
	busy: boolean;
	error?: string;
	change: (name: string, value: string) => void;
	restore: (name: string) => void;
} = $props();

const labels: Record<string, string> = {
	"theme.family": "主题",
	"theme.tone": "外观",
	google: "Google",
	"google-accounts": "Google Accounts",
	"google-workspace": "Google Workspace",
	material: "Material",
	system: "跟随设备",
	light: "浅色",
	dark: "深色",
};
const sources = {
	default: "默认设置",
	shared: "共享设置",
	app: "应用设置",
	policy: "策略",
};
</script>

{#each Object.values(view.items) as item (item.name)}
	<Preference
		name={item.name}
		label={labels[item.name] ?? item.name}
		value={item.effective}
		source={item.source}
		declaration={level === "app" ? item.app : item.shared}
		origin={sources[item.source]}
		effect={`当前生效：${labels[item.effective] ?? item.effective}`}
		choices={item.options.map((value) => ({ value, label: labels[value] ?? value }))}
		editable={item.editable}
		{busy}
		{error}
		inherit={level === "app" ? "继承共享或默认设置" : "恢复默认设置"}
		{look}
		change={(value) => change(item.name, value)}
		restore={() => restore(item.name)}
	/>
{/each}
