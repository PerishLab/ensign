<script lang="ts">
import {
	Aside,
	Button,
	Face,
	Frame,
	Head,
	Line,
	Menu,
	Nav,
	Navigator,
	Note,
	Shell,
	Text,
	Toast,
} from "@perishlab/design";
import { onMount, type Snippet } from "svelte";
import { type Actor, account, Fault } from "../../../account.ts";
import { folio, forget, shown, wear } from "../../../authorization.ts";
import { menu } from "../../../preferences.ts";
import { read, save, type Tone, watch } from "../../../tone.ts";
import { login, visit } from "../route.ts";

export type Entry = {
	label: string;
	href: string;
	live?: boolean;
	group?: string;
	sign?: string;
};

const initial = shown();

let {
	route,
	title,
	line,
	extra = [],
	system = initial?.system ?? "google-workspace",
	mode,
	shade,
	notes = [],
	mood = "calm",
	children,
}: {
	route: string;
	title: string;
	line?: string;
	extra?: Entry[];
	system?: string;
	mode?: Tone;
	shade?: (next: Tone) => void;
	notes?: string[];
	mood?: "calm" | "warn";
	children: Snippet<[Actor, (next: Actor) => void]>;
} = $props();

let who: Actor | undefined = $state();
let loading = $state(true);
let fault: Fault | undefined = $state();
let drawer = $state(false);
let compact = $state(false);
let tone: Tone = $state(initial?.tone ?? read());
let hub: string | undefined = $state();
let alive = true;

let active = $derived(mode ?? tone);
let links = $derived<Entry[]>([
	{ label: "概览", href: "/", sign: "places", group: "个人中心", live: route === "/" },
	{ label: "安全与恢复", href: "/security", sign: "owns", group: "个人中心", live: route === "/security" },
	{ label: "会话与令牌", href: "/sessions", sign: "bounds", group: "个人中心", live: route === "/sessions" },
	{ label: "偏好设置", href: "/settings", sign: "draws", group: "个人中心", live: route === "/settings" },
	{ label: "账号与服务", href: "/admin/actors", sign: "owns", group: "管理控制台", live: route === "/admin/actors" },
	{ label: "邀请管理", href: "/admin/invites", sign: "takes", group: "管理控制台", live: route === "/admin/invites" },
	{ label: "团队与成员", href: "/admin/teams", sign: "places", group: "管理控制台", live: route === "/admin/teams" },
	{ label: "应用登记", href: "/admin/apps", sign: "draws", group: "管理控制台", live: route === "/admin/apps" },
	...extra,
]);
let crumbs = $derived<Entry[]>(
	route === "/"
		? [{ label: "Ensign", href: "/" }, { label: "概览", href: "/", live: true }]
		: route.startsWith("/admin/")
			? [
					{ label: "Ensign", href: "/" },
					{ label: "管理控制台", href: "/admin/actors" },
					{ label: title, href: route, live: true },
				]
			: [{ label: "Ensign", href: "/" }, { label: title, href: route, live: true }],
);
let choices = $derived(menu(hub));

function toggle(): void {
	if (matchMedia("(max-width: 47.9375rem)").matches) drawer = !drawer;
	else compact = !compact;
}

function flip(): void {
	const next: Tone = active === "dark" ? "light" : "dark";
	tone = next;
	save(next);
	if (shown()) wear({ system, tone: next });
	shade?.(next);
}

function move(href: string): void {
	drawer = false;
	if (hub && href === hub) {
		globalThis.open(href, "_blank", "noopener,noreferrer");
		return;
	}
	if (href === "/logout") {
		globalThis.location.assign(href);
		return;
	}
	visit(href);
}

function update(next: Actor): void {
	who = next;
}

async function load(): Promise<void> {
	loading = true;
	fault = undefined;
	try {
		const actor = await account.whoami();
		if (alive) who = actor;
	} catch (error) {
		if (!alive) return;
		if (error instanceof Fault && error.status === 401) {
			forget();
			login(`${location.pathname}${location.search}`);
			return;
		}
		fault = error instanceof Fault ? error : new Fault(0, "read");
	} finally {
		if (alive) loading = false;
	}
}

onMount(() => {
	const stop = watch((next) => { tone = next; });
	void load();
	void folio().then((result) => {
		if (alive && result.status === "configured") hub = result.href;
	});
	return () => {
		alive = false;
		stop();
	};
});
</script>

<svelte:head><title>Ensign · {title}</title></svelte:head>

<Shell {system} tone={active}>
	<Navigator look="bar" stick mark="ensign" owner="PerishLab" title="Ensign" home="/">
		{#snippet lead()}
			<Button name="切换导航" look="quiet" sign="menu" press={toggle} />
		{/snippet}
		<Button name={active === "dark" ? "切换到浅色外观" : "切换到深色外观"} look="quiet" sign="shade" press={flip} />
		{#if who}
			{@const actor = who}
			<Menu label="账号菜单" kind="action" items={choices} choose={move}>
				{#snippet cue()}
					<Face name={actor.name} />
				{/snippet}
				{#snippet head()}
					<Line name={actor.name} meta={actor.login} />
				{/snippet}
			</Menu>
		{/if}
	</Navigator>
	<Frame look="full">
		<Aside under bind:open={drawer} rail={compact} label="个人中心导航">
			{#snippet side()}
				<Nav look="rail" {links} rail={compact && !drawer} label="主导航" pick={move} />
			{/snippet}
			<Nav look="crumb" links={crumbs} label="当前位置" pick={move} />
			<Head text={title} />
			{#if line}<Text>{line}</Text>{/if}
			{#if loading}
				<Note text="正在验证会话…" live />
			{:else if fault}
				<Note mood={fault.mood} code={fault.code} title="无法读取账号状态" text={fault.message} live>
					<Button label="重试" look="quiet" sign="sync" press={() => void load()} />
				</Note>
			{:else if who}
				{@render children(who, update)}
			{/if}
		</Aside>
	</Frame>
	{#if notes.length > 0}<Toast {notes} {mood} />{/if}
</Shell>
