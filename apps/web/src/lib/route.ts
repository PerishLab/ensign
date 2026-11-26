export function visit(href: string): void {
	if (href === globalThis.location?.pathname) return;
	history.pushState(null, "", href);
	dispatchEvent(new PopStateEvent("popstate"));
}

export function login(back: string): void {
	const target = `/login?return=${encodeURIComponent(back)}`;
	globalThis.location.assign(target);
}
